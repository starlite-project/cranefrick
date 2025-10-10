use inkwell::{
	AddressSpace,
	attributes::Attribute,
	context::{AsContextRef, Context, ContextRef},
	types::PointerType,
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
