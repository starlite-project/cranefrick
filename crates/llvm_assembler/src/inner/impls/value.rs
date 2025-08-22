use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn move_value(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_value = self.load(0)?;
		self.set_cell(0, 0)?;

		let other_cell = self.load(offset)?;

		let value_to_add = {
			let i8_type = self.context.i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(current_value, factor, "multiply current value by factor")
		}?;

		let added =
			self.builder
				.build_int_add(other_cell, value_to_add, "add value to other cell")?;

		self.store(added, offset)
	}

	pub fn take_value(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		self.move_value(factor, offset)?;
		self.move_pointer(offset)
	}

	pub fn fetch_value(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let other_cell = self.load(offset)?;

		self.set_cell(0, offset)?;

		let current_cell = self.load(0)?;

		let value_to_add = {
			let i8_type = self.context.i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(other_cell, factor, "multiply other value by factor")
		}?;

		let added =
			self.builder
				.build_int_add(current_cell, value_to_add, "add value to current cell")?;

		self.store(added, 0)
	}

	pub fn replace_value(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		self.set_cell(0, 0)?;
		self.fetch_value(factor, offset)
	}
}
