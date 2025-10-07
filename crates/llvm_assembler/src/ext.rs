use inkwell::{
	AddressSpace,
	attributes::Attribute,
	context::{Context, ContextRef},
	types::PointerType,
};

pub trait ContextExt {
	fn default_ptr_type(&self) -> PointerType<'_>;

	fn create_named_enum_attribute(&self, name: &'static str, val: u64) -> Attribute;
}

impl ContextExt for Context {
	fn default_ptr_type(&self) -> PointerType<'_> {
		self.ptr_type(AddressSpace::default())
	}

	fn create_named_enum_attribute(&self, name: &'static str, val: u64) -> Attribute {
		self.create_enum_attribute(Attribute::get_named_enum_kind_id(name), val)
	}
}

impl ContextExt for ContextRef<'_> {
	fn default_ptr_type(&self) -> PointerType<'_> {
		self.ptr_type(AddressSpace::default())
	}

	fn create_named_enum_attribute(&self, name: &'static str, val: u64) -> Attribute {
		self.create_enum_attribute(Attribute::get_named_enum_kind_id(name), val)
	}
}
