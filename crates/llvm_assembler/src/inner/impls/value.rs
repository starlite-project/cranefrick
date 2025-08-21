use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn move_value(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_value = self.load(0)?;

		self.set_cell(0, 0)?;

		let other_cell = self.load(offset)?;

		let factor = {
			let i8_type = self.context.i8_type();

			i8_type.const_int(factor.into(), false)
		};

		let value_to_add = self
			.builder
			.build_int_mul(current_value, factor, "multiply value")?;

		let added = self
			.builder
			.build_int_add(other_cell, value_to_add, "add multiplied value")?;

		self.store(added, offset)
	}

	pub fn take_value(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		self.move_value(factor, offset)?;
		self.move_pointer(offset)
	}
}
