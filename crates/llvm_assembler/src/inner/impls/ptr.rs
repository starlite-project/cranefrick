use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn move_pointer(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let ptr_type = self.ptr_type;
		let offset = ptr_type.const_int(offset as u64, false);

		let offset_ptr  = {
			let load =self.builder.build_load(ptr_type, self.ptr, "load ptr")?.into_int_value();

			self.builder.build_int_add(load, offset, "offset ptr")
		}?;

		self.builder.build_store(self.ptr, offset_ptr)?;

		Ok(())
	}
}
