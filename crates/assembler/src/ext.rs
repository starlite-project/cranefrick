#![allow(dead_code)]

use std::{
	borrow::Cow,
	ffi::{CStr, CString},
};

use frick_utils::Convert as _;
use inkwell::{
	AddressSpace,
	attributes::Attribute,
	builder::{Builder, BuilderError},
	context::{AsContextRef, Context, ContextRef},
	debug_info::{
		DIBasicType, DICompileUnit, DICompositeType, DIDerivedType, DIExpression, DIFile,
		DILexicalBlock, DILocalVariable, DILocation, DINamespace, DIScope, DIType,
	},
	llvm_sys::{
		LLVMGEPNoWrapFlags,
		core::{
			LLVMBuildGEPWithNoWrapFlags, LLVMCreateConstantRangeAttribute, LLVMIsNewDbgInfoFormat,
			LLVMSetIsNewDbgInfoFormat,
		},
		target_machine::{LLVMSetTargetMachineFastISel, LLVMSetTargetMachineGlobalISel},
	},
	module::Module,
	targets::TargetMachine,
	types::{BasicType, PointerType},
	values::{AsValueRef, IntValue, MetadataValue, PointerValue},
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

	fn create_range_attribute(
		&self,
		kind_id: u32,
		num_bits: u32,
		lower_bound: u64,
		upper_bound: u64,
	) -> Attribute;
}

impl<'ctx> ContextExt<'ctx> for &'ctx Context {
	fn default_ptr_type(&self) -> PointerType<'ctx> {
		self.ptr_type(AddressSpace::default())
	}

	fn create_named_enum_attribute(&self, name: &'static str, val: u64) -> Attribute {
		self.create_enum_attribute(Attribute::get_named_enum_kind_id(name), val)
	}

	fn create_range_attribute(
		&self,
		kind_id: u32,
		num_bits: u32,
		lower_bound: u64,
		upper_bound: u64,
	) -> Attribute {
		unsafe {
			Attribute::new(LLVMCreateConstantRangeAttribute(
				self.raw(),
				kind_id,
				num_bits,
				&raw const lower_bound,
				&raw const upper_bound,
			))
		}
	}
}

impl<'ctx> ContextExt<'ctx> for ContextRef<'ctx> {
	fn default_ptr_type(&self) -> PointerType<'ctx> {
		self.ptr_type(AddressSpace::default())
	}

	fn create_named_enum_attribute(&self, name: &'static str, val: u64) -> Attribute {
		self.create_enum_attribute(Attribute::get_named_enum_kind_id(name), val)
	}

	fn create_range_attribute(
		&self,
		kind_id: u32,
		num_bits: u32,
		lower_bound: u64,
		upper_bound: u64,
	) -> Attribute {
		unsafe {
			Attribute::new(LLVMCreateConstantRangeAttribute(
				self.raw(),
				kind_id,
				num_bits,
				&raw const lower_bound,
				&raw const upper_bound,
			))
		}
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

pub trait TargetMachineExt {
	fn set_fast_instruction_selection(&self, value: bool);
}

impl TargetMachineExt for TargetMachine {
	fn set_fast_instruction_selection(&self, value: bool) {
		unsafe {
			LLVMSetTargetMachineFastISel(self.as_mut_ptr(), value.convert::<i32>());
		}
	}
}

pub trait BuilderExt<'ctx> {
	unsafe fn build_gep_with_no_wrap_flags<T: BasicType<'ctx>>(
		&self,
		pointee_ty: T,
		ptr: PointerValue<'ctx>,
		ordered_indexes: &[IntValue<'ctx>],
		name: &str,
		flags: LLVMGEPNoWrapFlags,
	) -> Result<PointerValue<'ctx>, BuilderError>;
}

impl<'ctx> BuilderExt<'ctx> for Builder<'ctx> {
	unsafe fn build_gep_with_no_wrap_flags<T: BasicType<'ctx>>(
		&self,
		pointee_ty: T,
		ptr: PointerValue<'ctx>,
		ordered_indexes: &[IntValue<'ctx>],
		name: &str,
		flags: LLVMGEPNoWrapFlags,
	) -> Result<PointerValue<'ctx>, BuilderError> {
		let c_string = to_c_string(name);

		let mut indexed_values = ordered_indexes
			.iter()
			.map(AsValueRef::as_value_ref)
			.collect::<Vec<_>>();

		let value = unsafe {
			LLVMBuildGEPWithNoWrapFlags(
				self.as_mut_ptr(),
				pointee_ty.as_type_ref(),
				ptr.as_value_ref(),
				indexed_values.as_mut_ptr(),
				indexed_values.len() as u32,
				c_string.as_ptr(),
				flags,
			)
		};

		Ok(unsafe { PointerValue::new(value) })
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
