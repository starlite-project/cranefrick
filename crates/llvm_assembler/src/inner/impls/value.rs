use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn move_value_to(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_value = self.load(0)?;
		self.set_cell(0, 0)?;

		let other_cell = self.load(offset)?;

		let value_to_add = {
			let i8_type = self.context.i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(current_value, factor, "move_value_mul")
		}?;

		let added = self
			.builder
			.build_int_add(other_cell, value_to_add, "move_value_add")?;

		self.store(added, offset)
	}

	pub fn take_value_to(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_value = self.load(0)?;
		self.set_cell(0, 0)?;

		self.move_pointer(offset)?;

		let other_cell = self.load(0)?;

		let value_to_add = {
			let i8_type = self.context.i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(current_value, factor, "take_value_mul")
		}?;

		let added = self
			.builder
			.build_int_add(other_cell, value_to_add, "take_value_add")?;

		self.store(added, 0)
	}

	pub fn fetch_value_from(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let other_cell = self.load(offset)?;

		self.set_cell(0, offset)?;

		let current_cell = self.load(0)?;

		let value_to_add = {
			let i8_type = self.context.i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(other_cell, factor, "fetch_value_mul")
		}?;

		let added = self
			.builder
			.build_int_add(current_cell, value_to_add, "fetch_value_add")?;

		self.store(added, 0)
	}

	pub fn replace_value_from(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let other_cell = self.load(offset)?;

		self.set_cell(0, offset)?;

		let value_to_store = {
			let i8_type = self.context.i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(other_cell, factor, "replace_value_mul")
		}?;

		self.store(value_to_store, 0)
	}
}
