#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![allow(clippy::result_large_err)]

mod data_context;
mod serde_impl;
mod traps;

use std::{
	borrow::Cow,
	error::Error as StdError,
	fmt::{Debug, Display, Formatter, Result as FmtResult, Write as _},
	io::Error as IoError,
};

use cranelift_codegen::{
	CodegenError, CompileError, Context,
	binemit::{CodeOffset, Reloc},
	entity::{PrimaryMap, entity_impl},
	ir::{self, function::VersionMarker},
	isa,
	settings::SetError,
};
use cranelift_control::ControlPlane;
use hashbrown::{HashMap, hash_map::Entry};
use serde::{Deserialize, Serialize};

pub use self::{data_context::*, traps::TrapSite};

#[derive(Clone)]
pub struct ModuleReloc {
	pub offset: CodeOffset,
	pub kind: Reloc,
	pub name: ModuleRelocTarget,
	pub addend: i64,
}

impl ModuleReloc {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct FuncId(u32);

entity_impl!(FuncId, "funcid");

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct DataId(u32);

entity_impl!(DataId, "dataid");

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionDeclaration {
	pub name: Option<String>,
	pub linkage: Linkage,
	pub signature: ir::Signature,
}

impl FunctionDeclaration {
	#[must_use]
	pub fn linkage_name(&self, id: FuncId) -> Cow<'_, str> {
		match &self.name {
			Some(name) => Cow::Borrowed(name),
			None => Cow::Owned(format!(".Lfn{:x}", id.as_u32())),
		}
	}

	fn merge(
		&mut self,
		id: FuncId,
		linkage: Linkage,
		sig: &ir::Signature,
	) -> Result<(), ModuleError> {
		self.linkage = Linkage::merge(self.linkage, linkage);
		if &self.signature != sig {
			return Err(ModuleError::IncompatibleSignature(
				self.linkage_name(id).into_owned(),
				self.signature.clone(),
				sig.clone(),
			));
		}

		Ok(())
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataDeclaration {
	pub name: Option<String>,
	pub linkage: Linkage,
	pub writable: bool,
	pub tls: bool,
}

impl DataDeclaration {
	#[must_use]
	pub fn linkage_name(&self, id: DataId) -> Cow<'_, str> {
		match &self.name {
			Some(name) => Cow::Borrowed(name),
			None => Cow::Owned(format!(".Ldata{:x}", id.as_u32())),
		}
	}

	fn merge(&mut self, linkage: Linkage, writable: bool, tls: bool) {
		self.linkage = Linkage::merge(self.linkage, linkage);
		self.writable = self.writable || writable;
		assert_eq!(
			self.tls, tls,
			"can't change TLS data object to normal or in the opposite way"
		);
	}
}

#[derive(Debug, Default)]
pub struct ModuleDeclarations {
	_version_marker: VersionMarker,
	names: HashMap<String, FuncOrDataId>,
	functions: PrimaryMap<FuncId, FunctionDeclaration>,
	data_objects: PrimaryMap<DataId, DataDeclaration>,
}

impl ModuleDeclarations {
	#[must_use]
	pub fn get_name(&self, name: &str) -> Option<FuncOrDataId> {
		self.names.get(name).copied()
	}

	#[must_use]
	pub fn get_functions(
		&self,
	) -> cranelift_codegen::entity::Iter<'_, FuncId, FunctionDeclaration> {
		self.functions.iter()
	}

	#[must_use]
	pub const fn is_function(name: &ModuleRelocTarget) -> bool {
		match name {
			ModuleRelocTarget::User { namespace, .. } => matches!(namespace, 0),
			_ => panic!("unexpected module ext name"),
		}
	}

	#[must_use]
	pub fn get_function_decl(&self, func_id: FuncId) -> &FunctionDeclaration {
		&self.functions[func_id]
	}

	#[must_use]
	pub fn get_data_objects(&self) -> cranelift_codegen::entity::Iter<'_, DataId, DataDeclaration> {
		self.data_objects.iter()
	}

	#[must_use]
	pub fn get_data_decl(&self, data_id: DataId) -> &DataDeclaration {
		&self.data_objects[data_id]
	}

	pub fn declare_function(
		&mut self,
		name: String,
		linkage: Linkage,
		signature: ir::Signature,
	) -> Result<(FuncId, Linkage), ModuleError> {
		match self.names.entry(name.clone()) {
			Entry::Occupied(entry) => match *entry.get() {
				FuncOrDataId::Data(..) => Err(ModuleError::IncompatibleDeclaration(name)),
				FuncOrDataId::Func(id) => {
					let existing = &mut self.functions[id];
					existing.merge(id, linkage, &signature)?;
					Ok((id, existing.linkage))
				}
			},
			Entry::Vacant(entry) => {
				let id = self.functions.push(FunctionDeclaration {
					name: Some(name),
					linkage,
					signature,
				});
				entry.insert(FuncOrDataId::Func(id));
				Ok((id, self.functions[id].linkage))
			}
		}
	}

	pub fn declare_anonymous_function(
		&mut self,
		signature: ir::Signature,
	) -> Result<FuncId, ModuleError> {
		Ok(self.functions.push(FunctionDeclaration {
			name: None,
			linkage: Linkage::Local,
			signature,
		}))
	}

	pub fn declare_data(
		&mut self,
		name: String,
		linkage: Linkage,
		writable: bool,
		tls: bool,
	) -> Result<(DataId, Linkage), ModuleError> {
		match self.names.entry(name.clone()) {
			Entry::Occupied(entry) => match *entry.get() {
				FuncOrDataId::Data(id) => {
					let existing = &mut self.data_objects[id];
					existing.merge(linkage, writable, tls);
					Ok((id, existing.linkage))
				}
				FuncOrDataId::Func(..) => Err(ModuleError::IncompatibleDeclaration(name)),
			},
			Entry::Vacant(entry) => {
				let id = self.data_objects.push(DataDeclaration {
					name: Some(name),
					linkage,
					writable,
					tls,
				});
				entry.insert(FuncOrDataId::Data(id));
				Ok((id, self.data_objects[id].linkage))
			}
		}
	}

	pub fn declare_anonymous_data(
		&mut self,
		writable: bool,
		tls: bool,
	) -> Result<DataId, ModuleError> {
		Ok(self.data_objects.push(DataDeclaration {
			name: None,
			linkage: Linkage::Local,
			writable,
			tls,
		}))
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleRelocTarget {
	User { namespace: u32, index: u32 },
	LibCall(ir::LibCall),
	KnownSymbol(ir::KnownSymbol),
	FunctionOffset(FuncId, CodeOffset),
}

impl ModuleRelocTarget {
	#[must_use]
	pub const fn user(namespace: u32, index: u32) -> Self {
		Self::User { namespace, index }
	}
}

impl Display for ModuleRelocTarget {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::User { namespace, index } => {
				f.write_char('u')?;
				Display::fmt(&namespace, f)?;
				f.write_char(':')?;
				Display::fmt(&index, f)
			}
			Self::LibCall(lc) => {
				f.write_char('%')?;
				Display::fmt(&lc, f)
			}
			Self::KnownSymbol(ks) => Display::fmt(&ks, f),
			Self::FunctionOffset(fname, offset) => {
				Display::fmt(&fname, f)?;
				f.write_char('+')?;
				Display::fmt(&offset, f)
			}
		}
	}
}

impl From<FuncId> for ModuleRelocTarget {
	fn from(value: FuncId) -> Self {
		Self::User {
			namespace: 0,
			index: value.0,
		}
	}
}

impl From<DataId> for ModuleRelocTarget {
	fn from(value: DataId) -> Self {
		Self::User {
			namespace: 1,
			index: value.0,
		}
	}
}

impl From<FuncOrDataId> for ModuleRelocTarget {
	fn from(value: FuncOrDataId) -> Self {
		match value {
			FuncOrDataId::Data(id) => Self::from(id),
			FuncOrDataId::Func(id) => Self::from(id),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Linkage {
	Import,
	Local,
	Preemtible,
	Hidden,
	Export,
}

impl Linkage {
	const fn merge(a: Self, b: Self) -> Self {
		match (a, b) {
			(Self::Hidden | Self::Local, Self::Preemtible) | (Self::Preemtible, ..) => {
				Self::Preemtible
			}
			(Self::Hidden, ..) | (Self::Local, Self::Hidden) => Self::Hidden,
			(Self::Export, ..) | (Self::Local, Self::Export) => Self::Export,
			(Self::Local, Self::Local | Self::Import) => Self::Local,
			(Self::Import, b) => b,
		}
	}

	#[must_use]
	pub const fn is_definable(self) -> bool {
		matches!(
			self,
			Self::Local | Self::Preemtible | Self::Hidden | Self::Export
		)
	}

	#[must_use]
	pub const fn is_final(self) -> bool {
		matches!(self, Self::Local | Self::Hidden | Self::Export)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FuncOrDataId {
	Func(FuncId),
	Data(DataId),
}

#[derive(Debug)]
pub enum ModuleError {
	Undeclared(String),
	IncompatibleDeclaration(String),
	IncompatibleSignature(String, ir::Signature, ir::Signature),
	DuplicateDefinition(String),
	InvalidImportDefinition(String),
	Compilation(CodegenError),
	Allocation {
		message: &'static str,
		source: IoError,
	},
	Backend(Box<dyn StdError + Send + Sync>),
	Flag(SetError),
}

impl Display for ModuleError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Undeclared(name) => {
				f.write_str("undeclared identifier: ")?;
				f.write_str(name)
			}
			Self::IncompatibleDeclaration(name) => {
				f.write_str("incompatible declaration of identifier: ")?;
				f.write_str(name)
			}
			Self::IncompatibleSignature(name, prev_sig, next_sig) => {
				f.write_str("function ")?;
				f.write_str(name)?;
				f.write_str(" signature ")?;
				Debug::fmt(&next_sig, f)?;
				f.write_str(" is incompatible with previous declaration ")?;
				Debug::fmt(&prev_sig, f)
			}
			Self::DuplicateDefinition(name) => {
				f.write_str("duplicate definition of identifier: ")?;
				f.write_str(name)
			}
			Self::InvalidImportDefinition(name) => {
				f.write_str("invalid to define identifier declared as an import: ")?;
				f.write_str(name)
			}
			Self::Compilation(..) => f.write_str("compilation error"),
			Self::Allocation { message, .. } => {
				f.write_str("allocation error: ")?;
				f.write_str(message)
			}
			Self::Backend(..) => f.write_str("backend error"),
			Self::Flag(..) => f.write_str("flag error"),
		}
	}
}

impl StdError for ModuleError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Undeclared(..)
			| Self::IncompatibleDeclaration(..)
			| Self::IncompatibleSignature(..)
			| Self::DuplicateDefinition(..)
			| Self::InvalidImportDefinition(..) => None,
			Self::Compilation(e) => Some(e),
			Self::Allocation { source, .. } => Some(source),
			Self::Backend(e) => Some(&**e),
			Self::Flag(e) => Some(e),
		}
	}
}

impl<'a> From<CompileError<'a>> for ModuleError {
	fn from(value: CompileError<'a>) -> Self {
		Self::from(value.inner)
	}
}

impl From<CodegenError> for ModuleError {
	fn from(value: CodegenError) -> Self {
		Self::Compilation(value)
	}
}

impl From<SetError> for ModuleError {
	fn from(value: SetError) -> Self {
		Self::Flag(value)
	}
}

pub trait Module {
	fn isa(&self) -> &dyn isa::TargetIsa;

	fn declarations(&self) -> &ModuleDeclarations;

	fn get_name(&self, name: &str) -> Option<FuncOrDataId> {
		self.declarations().get_name(name)
	}

	fn target_config(&self) -> isa::TargetFrontendConfig {
		self.isa().frontend_config()
	}

	fn make_context(&self) -> Context {
		let mut ctx = Context::new();
		ctx.func.signature.call_conv = self.isa().default_call_conv();
		ctx
	}

	fn clear_context(&self, ctx: &mut Context) {
		ctx.clear();
		ctx.func.signature.call_conv = self.isa().default_call_conv();
	}

	fn make_signature(&self) -> ir::Signature {
		ir::Signature::new(self.isa().default_call_conv())
	}

	fn clear_signature(&self, sig: &mut ir::Signature) {
		sig.clear(self.isa().default_call_conv());
	}

	fn declare_function(
		&mut self,
		name: String,
		linkage: Linkage,
		signature: ir::Signature,
	) -> Result<FuncId, ModuleError>;

	fn declare_anonymous_function(
		&mut self,
		signature: ir::Signature,
	) -> Result<FuncId, ModuleError>;

	fn declare_data(
		&mut self,
		name: String,
		linkage: Linkage,
		writable: bool,
		tls: bool,
	) -> Result<DataId, ModuleError>;

	fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> Result<DataId, ModuleError>;

	fn declare_func_in_func(&mut self, func_id: FuncId, func: &mut ir::Function) -> ir::FuncRef {
		let decl = &self.declarations().functions[func_id];
		let signature = func.import_signature(decl.signature.clone());
		let user_name_ref = func.declare_imported_user_function(ir::UserExternalName {
			namespace: 0,
			index: func_id.as_u32(),
		});
		let colocated = decl.linkage.is_final();
		func.import_function(ir::ExtFuncData {
			name: ir::ExternalName::user(user_name_ref),
			signature,
			colocated,
		})
	}

	fn declare_data_in_func(&self, data: DataId, func: &mut ir::Function) -> ir::GlobalValue {
		let decl = &self.declarations().data_objects[data];
		let colocated = decl.linkage.is_final();
		let user_name_ref = func.declare_imported_user_function(ir::UserExternalName {
			namespace: 1,
			index: data.as_u32(),
		});

		func.create_global_value(ir::GlobalValueData::Symbol {
			name: ir::ExternalName::user(user_name_ref),
			offset: ir::immediates::Imm64::new(0),
			colocated,
			tls: decl.tls,
		})
	}

	fn declare_func_in_data(&self, func_id: FuncId, data: &mut DataDescription) -> ir::FuncRef {
		data.import_function(ModuleRelocTarget::user(0, func_id.as_u32()))
	}

	fn declare_data_in_data(&self, data_id: DataId, data: &mut DataDescription) -> ir::GlobalValue {
		data.import_global_value(ModuleRelocTarget::user(1, data_id.as_u32()))
	}

	fn define_function(&mut self, func: FuncId, ctx: &mut Context) -> Result<(), ModuleError> {
		self.define_function_with_control_plane(func, ctx, &mut ControlPlane::default())
	}

	fn define_function_with_control_plane(
		&mut self,
		func: FuncId,
		ctx: &mut Context,
		ctrl_plane: &mut ControlPlane,
	) -> Result<(), ModuleError>;

	fn define_function_bytes(
		&mut self,
		func_id: FuncId,
		alignment: u64,
		bytes: &[u8],
		relocs: &[ModuleReloc],
	) -> Result<(), ModuleError>;

	fn define_data(&mut self, data_id: DataId, data: &DataDescription) -> Result<(), ModuleError>;
}

impl<M> Module for &mut M
where
	M: ?Sized + Module,
{
	fn isa(&self) -> &dyn isa::TargetIsa {
		(**self).isa()
	}

	fn declarations(&self) -> &ModuleDeclarations {
		(**self).declarations()
	}

	fn get_name(&self, name: &str) -> Option<FuncOrDataId> {
		(**self).get_name(name)
	}

	fn target_config(&self) -> isa::TargetFrontendConfig {
		(**self).target_config()
	}

	fn make_context(&self) -> Context {
		(**self).make_context()
	}

	fn clear_context(&self, ctx: &mut Context) {
		(**self).clear_context(ctx);
	}

	fn make_signature(&self) -> ir::Signature {
		(**self).make_signature()
	}

	fn clear_signature(&self, sig: &mut ir::Signature) {
		(**self).clear_signature(sig);
	}

	fn declare_function(
		&mut self,
		name: String,
		linkage: Linkage,
		signature: ir::Signature,
	) -> Result<FuncId, ModuleError> {
		(**self).declare_function(name, linkage, signature)
	}

	fn declare_anonymous_function(
		&mut self,
		signature: ir::Signature,
	) -> Result<FuncId, ModuleError> {
		(**self).declare_anonymous_function(signature)
	}

	fn declare_data(
		&mut self,
		name: String,
		linkage: Linkage,
		writable: bool,
		tls: bool,
	) -> Result<DataId, ModuleError> {
		(**self).declare_data(name, linkage, writable, tls)
	}

	fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> Result<DataId, ModuleError> {
		(**self).declare_anonymous_data(writable, tls)
	}

	fn declare_func_in_func(&mut self, func_id: FuncId, func: &mut ir::Function) -> ir::FuncRef {
		(**self).declare_func_in_func(func_id, func)
	}

	fn declare_data_in_func(&self, data: DataId, func: &mut ir::Function) -> ir::GlobalValue {
		(**self).declare_data_in_func(data, func)
	}

	fn declare_func_in_data(&self, func_id: FuncId, data: &mut DataDescription) -> ir::FuncRef {
		(**self).declare_func_in_data(func_id, data)
	}

	fn declare_data_in_data(&self, data_id: DataId, data: &mut DataDescription) -> ir::GlobalValue {
		(**self).declare_data_in_data(data_id, data)
	}

	fn define_function(&mut self, func: FuncId, ctx: &mut Context) -> Result<(), ModuleError> {
		(**self).define_function(func, ctx)
	}

	fn define_function_with_control_plane(
		&mut self,
		func: FuncId,
		ctx: &mut Context,
		ctrl_plane: &mut ControlPlane,
	) -> Result<(), ModuleError> {
		(**self).define_function_with_control_plane(func, ctx, ctrl_plane)
	}

	fn define_function_bytes(
		&mut self,
		func_id: FuncId,
		alignment: u64,
		bytes: &[u8],
		relocs: &[ModuleReloc],
	) -> Result<(), ModuleError> {
		(**self).define_function_bytes(func_id, alignment, bytes, relocs)
	}

	fn define_data(&mut self, data_id: DataId, data: &DataDescription) -> Result<(), ModuleError> {
		(**self).define_data(data_id, data)
	}
}

impl<M> Module for Box<M>
where
	M: ?Sized + Module,
{
	fn isa(&self) -> &dyn isa::TargetIsa {
		(**self).isa()
	}

	fn declarations(&self) -> &ModuleDeclarations {
		(**self).declarations()
	}

	fn get_name(&self, name: &str) -> Option<FuncOrDataId> {
		(**self).get_name(name)
	}

	fn target_config(&self) -> isa::TargetFrontendConfig {
		(**self).target_config()
	}

	fn make_context(&self) -> Context {
		(**self).make_context()
	}

	fn clear_context(&self, ctx: &mut Context) {
		(**self).clear_context(ctx);
	}

	fn make_signature(&self) -> ir::Signature {
		(**self).make_signature()
	}

	fn clear_signature(&self, sig: &mut ir::Signature) {
		(**self).clear_signature(sig);
	}

	fn declare_function(
		&mut self,
		name: String,
		linkage: Linkage,
		signature: ir::Signature,
	) -> Result<FuncId, ModuleError> {
		(**self).declare_function(name, linkage, signature)
	}

	fn declare_anonymous_function(
		&mut self,
		signature: ir::Signature,
	) -> Result<FuncId, ModuleError> {
		(**self).declare_anonymous_function(signature)
	}

	fn declare_data(
		&mut self,
		name: String,
		linkage: Linkage,
		writable: bool,
		tls: bool,
	) -> Result<DataId, ModuleError> {
		(**self).declare_data(name, linkage, writable, tls)
	}

	fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> Result<DataId, ModuleError> {
		(**self).declare_anonymous_data(writable, tls)
	}

	fn declare_func_in_func(&mut self, func_id: FuncId, func: &mut ir::Function) -> ir::FuncRef {
		(**self).declare_func_in_func(func_id, func)
	}

	fn declare_data_in_func(&self, data: DataId, func: &mut ir::Function) -> ir::GlobalValue {
		(**self).declare_data_in_func(data, func)
	}

	fn declare_func_in_data(&self, func_id: FuncId, data: &mut DataDescription) -> ir::FuncRef {
		(**self).declare_func_in_data(func_id, data)
	}

	fn declare_data_in_data(&self, data_id: DataId, data: &mut DataDescription) -> ir::GlobalValue {
		(**self).declare_data_in_data(data_id, data)
	}

	fn define_function(&mut self, func: FuncId, ctx: &mut Context) -> Result<(), ModuleError> {
		(**self).define_function(func, ctx)
	}

	fn define_function_with_control_plane(
		&mut self,
		func: FuncId,
		ctx: &mut Context,
		ctrl_plane: &mut ControlPlane,
	) -> Result<(), ModuleError> {
		(**self).define_function_with_control_plane(func, ctx, ctrl_plane)
	}

	fn define_function_bytes(
		&mut self,
		func_id: FuncId,
		alignment: u64,
		bytes: &[u8],
		relocs: &[ModuleReloc],
	) -> Result<(), ModuleError> {
		(**self).define_function_bytes(func_id, alignment, bytes, relocs)
	}

	fn define_data(&mut self, data_id: DataId, data: &DataDescription) -> Result<(), ModuleError> {
		(**self).define_data(data_id, data)
	}
}

#[must_use]
pub fn default_libcall_names() -> Box<dyn Fn(ir::LibCall) -> String + Send + Sync> {
	Box::new(move |libcall| {
		match libcall {
			ir::LibCall::Probestack => "__cranelift_probestack",
			ir::LibCall::CeilF32 => "ceilf",
			ir::LibCall::CeilF64 => "ceil",
			ir::LibCall::FloorF32 => "floorf",
			ir::LibCall::FloorF64 => "floor",
			ir::LibCall::TruncF32 => "truncf",
			ir::LibCall::TruncF64 => "trunc",
			ir::LibCall::NearestF32 => "nearbyintf",
			ir::LibCall::NearestF64 => "nearbyint",
			ir::LibCall::FmaF32 => "fmaf",
			ir::LibCall::FmaF64 => "fma",
			ir::LibCall::Memcpy => "memcpy",
			ir::LibCall::Memset => "memset",
			ir::LibCall::Memmove => "memmove",
			ir::LibCall::Memcmp => "memcmp",
			ir::LibCall::ElfTlsGetAddr => "__tls_get_addr",
			ir::LibCall::ElfTlsGetOffset => "__tls_get_offset",
			ir::LibCall::X86Pshufb => "__cranelift_x86_pshufb",
		}
		.to_owned()
	})
}
