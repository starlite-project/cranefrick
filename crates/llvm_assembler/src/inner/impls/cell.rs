use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn set_cell(&self, value: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context.i8_type();

		let value = i8_type.const_int(u64::from(value), false);

		self.store(value, offset)
	}

	pub fn change_cell(&self, value: i8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context.i8_type();

		let value = i8_type.const_int(value as u64, false);

		let current_value = self.load(offset)?;

		let new_value = self
			.builder
			.build_int_add(value, current_value, "add values")?;

		self.store(new_value, offset)
	}

	pub fn sub_cell(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_cell = self.load(0)?;

		self.set_cell(0, 0)?;

		let other_value = self.load(offset)?;

		let value_to_store = self.builder.build_int_sub(other_value, current_cell, "sub cells")?;

		self.store(value_to_store, offset)
	}
}
