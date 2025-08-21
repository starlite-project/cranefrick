use crate::{ContextExt, LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn set_cell(&self, value: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		// let ptr_type = self.context.default_ptr_type();

		// let i8_type = self.context.i8_type();
		// let i8_amount = i8_type.const_int(value as u64, false);

		// let ptr_load = self.builder.build_load(ptr_type, self.ptr, "load ptr")?.into_pointer_value();

		// self.builder.build_store(ptr_load, i8_amount)?;

		let value = self.context.i8_type().const_int(u64::from(value), false);

		self.store(value, offset)?;

		Ok(())
	}

	pub fn change_cell(&self, value: i8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_cell_value = self.load(offset)?;

		let value_to_add = self.context.i8_type().const_int(value as u64, false);

		let added_together = self.builder.build_int_add(current_cell_value, value_to_add, "add to value")?;

		self.store(added_together, offset)?;

		Ok(())
	}
}
