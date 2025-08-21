use inkwell::{AddressSpace, context::Context, types::PointerType};

pub trait ContextExt {
	fn default_ptr_type(&self) -> PointerType<'_>;
}

impl ContextExt for Context {
	fn default_ptr_type(&self) -> PointerType<'_> {
		self.ptr_type(AddressSpace::default())
	}
}
