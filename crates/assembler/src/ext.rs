#![allow(dead_code)]

use inkwell::{
	AddressSpace,
	attributes::Attribute,
	context::{AsContextRef, Context, ContextRef},
	debug_info::{
		DIBasicType, DICompileUnit, DICompositeType, DIDerivedType, DIExpression, DIFile,
		DILexicalBlock, DILocalVariable, DILocation, DINamespace, DIScope, DIType,
	},
	llvm_sys::core::{LLVMIsNewDbgInfoFormat, LLVMSetIsNewDbgInfoFormat},
	module::Module,
	types::PointerType,
	values::MetadataValue,
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
			LLVMSetIsNewDbgInfoFormat(self.as_mut_ptr(), value.into());
		}
	}

	fn is_new_debug_format(&self) -> bool {
		!matches!(unsafe { LLVMIsNewDbgInfoFormat(self.as_mut_ptr()) }, 0)
	}
}
