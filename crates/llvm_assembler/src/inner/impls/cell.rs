use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn set_cell(&self, value: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context.i8_type();

		let value = i8_type.const_int(value.into(), false);

		self.store(value, offset)
	}

	pub fn change_cell(&self, value: i8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context.i8_type();

		let value = i8_type.const_int(value as u64, false);

		let current_value_in_cell = self.load(offset)?;

		let added_values =
			self.builder
				.build_int_add(current_value_in_cell, value, "add to cell")?;

		self.store(added_values, offset)
	}
}
