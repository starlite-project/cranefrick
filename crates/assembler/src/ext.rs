#![allow(dead_code)]

use std::{
	borrow::Cow,
	ffi::{CStr, CString},
};

use frick_utils::Convert as _;
use inkwell::{
	AddressSpace,
	attributes::Attribute,
	basic_block::BasicBlock,
	builder::{Builder, BuilderError},
	context::{AsContextRef, Context, ContextRef},
	debug_info::{
		DIBasicType, DICompileUnit, DICompositeType, DIDerivedType, DIExpression, DIFile,
		DILexicalBlock, DILocalVariable, DILocation, DINamespace, DIScope, DIType,
		DebugInfoBuilder,
	},
	llvm_sys::{
		core::{LLVMBuildGEP2, LLVMIsNewDbgInfoFormat, LLVMSetIsNewDbgInfoFormat},
		debuginfo::LLVMDIBuilderInsertDbgValueRecordAtEnd,
		prelude::LLVMValueRef,
	},
	module::Module,
	types::{BasicType, PointerType},
	values::{
		AsValueRef, BasicValueEnum, InstructionValue, MetadataValue, PointerValue, VectorValue,
	},
};

pub trait ContextGetter<'ctx> {
	fn context(&self) -> ContextRef<'ctx>;
}

impl<'ctx, C> ContextGetter<'ctx> for C
where
	C: AsContextRef<'ctx>,
{
	fn context(&self) -> ContextRef<'ctx> {
		let raw_context_ref = self.as_ctx_ref();

		unsafe { ContextRef::new(raw_context_ref) }
	}
}

pub trait ContextExt<'ctx> {
	fn default_ptr_type(&self) -> PointerType<'ctx>;

	fn create_named_enum_attribute(&self, name: &'static str, val: u64) -> Attribute;
}

impl<'ctx> ContextExt<'ctx> for &'ctx Context {
	fn default_ptr_type(&self) -> PointerType<'ctx> {
		self.ptr_type(AddressSpace::default())
	}

	fn create_named_enum_attribute(&self, name: &'static str, val: u64) -> Attribute {
		self.create_enum_attribute(Attribute::get_named_enum_kind_id(name), val)
	}
}

impl<'ctx> ContextExt<'ctx> for ContextRef<'ctx> {
	fn default_ptr_type(&self) -> PointerType<'ctx> {
		self.ptr_type(AddressSpace::default())
	}

	fn create_named_enum_attribute(&self, name: &'static str, val: u64) -> Attribute {
		self.create_enum_attribute(Attribute::get_named_enum_kind_id(name), val)
	}
}

// Until 607 (https://github.com/TheDan64/inkwell/pull/607) lands, this is here to extend things
pub trait DIExt<'ctx> {
	fn as_metadata_value(&self, context: impl AsContextRef<'ctx>) -> MetadataValue<'ctx>;
}

macro_rules! impl_di_ext {
	($($ty:ty),*) => {
		$(
			impl<'ctx> $crate::ext::DIExt<'ctx> for $ty {
				fn as_metadata_value(&self, context: impl ::inkwell::context::AsContextRef<'ctx>) -> ::inkwell::values::MetadataValue<'ctx> {
					unsafe { ::inkwell::values::MetadataValue::new(::inkwell::llvm_sys::core::LLVMMetadataAsValue(context.as_ctx_ref(), self.as_mut_ptr())) }
				}
			}
		)*
	};
}

impl_di_ext!(
	DIScope<'ctx>,
	DIFile<'ctx>,
	DINamespace<'ctx>,
	DICompileUnit<'ctx>,
	DIType<'ctx>,
	DIDerivedType<'ctx>,
	DICompositeType<'ctx>,
	DIBasicType<'ctx>,
	DILexicalBlock<'ctx>,
	DILocation<'ctx>,
	DILocalVariable<'ctx>,
	DIExpression<'ctx>
);

pub trait ModuleExt<'ctx> {
	fn set_new_debug_format(&self, value: bool);

	fn is_new_debug_format(&self) -> bool;
}

impl<'ctx> ModuleExt<'ctx> for Module<'ctx> {
	fn set_new_debug_format(&self, value: bool) {
		unsafe {
			LLVMSetIsNewDbgInfoFormat(self.as_mut_ptr(), value.convert::<i32>());
		}
	}

	fn is_new_debug_format(&self) -> bool {
		!matches!(unsafe { LLVMIsNewDbgInfoFormat(self.as_mut_ptr()) }, 0)
	}
}

pub trait BuilderExt<'ctx> {
	unsafe fn build_vec_gep<T: BasicType<'ctx>>(
		&self,
		pointee_ty: T,
		ptr: PointerValue<'ctx>,
		vec_of_indices: VectorValue<'ctx>,
		name: &str,
	) -> Result<VectorValue<'ctx>, BuilderError>;
}

impl<'ctx> BuilderExt<'ctx> for Builder<'ctx> {
	unsafe fn build_vec_gep<T: BasicType<'ctx>>(
		&self,
		pointee_ty: T,
		ptr: PointerValue<'ctx>,
		vec_of_indices: VectorValue<'ctx>,
		name: &str,
	) -> Result<VectorValue<'ctx>, BuilderError> {
		let c_string = to_c_string(name);

		let mut index_values = [vec_of_indices.as_value_ref()];

		let value = unsafe {
			LLVMBuildGEP2(
				self.as_mut_ptr(),
				pointee_ty.as_type_ref(),
				ptr.as_value_ref(),
				index_values.as_mut_ptr(),
				index_values.len() as u32,
				c_string.as_ptr(),
			)
		};

		Ok(unsafe { VectorValue::new(value) })
	}
}

fn to_c_string(mut s: &str) -> Cow<'_, CStr> {
	if s.is_empty() {
		s = "\0";
	}

	if !s.chars().rev().any(|ch| matches!(ch, '\0')) {
		return Cow::from(CString::new(s).expect("unreachable since null bytes are checked"));
	}

	Cow::from(unsafe { CStr::from_ptr(s.as_ptr().cast()) })
}
