use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn move_pointer(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i32_type =self.context.i32_type();
		let offset = i32_type.const_int(offset as u64, false);

		let ptr  = {
			let load =self.builder.build_load(i32_type, self.ptr, "load ptr")?.into_int_value();

			self.builder.build_int_add(load, offset, "offset ptr")
		}?;

		self.builder.build_store(self.ptr, ptr)?;

		Ok(())
	}
}
