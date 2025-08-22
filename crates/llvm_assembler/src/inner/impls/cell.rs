use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn set_cell(&self, value: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let value = self.context.i8_type().const_int(u64::from(value), false);

		self.store(value, offset)?;

		Ok(())
	}

	pub fn change_cell(&self, value: i8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_cell_value = self.load(offset)?;

		let value_to_add = self.context.i8_type().const_int(value as u64, false);

		let added_together =
			self.builder
				.build_int_add(current_cell_value, value_to_add, "add to value")?;

		self.store(added_together, offset)?;

		Ok(())
	}

	pub fn sub_cell(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let subtractor = self.load(0)?;

		self.set_cell(0, 0)?;

		let other_value = self.load(offset)?;

		let value_to_store = self.builder.build_int_sub(
			other_value,
			subtractor,
			"sub other_value by current value",
		)?;

		self.store(value_to_store, offset)
	}
}
