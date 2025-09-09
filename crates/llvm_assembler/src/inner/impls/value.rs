use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn move_value_to(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_value = self.load(0, "move_value_to")?;
		self.store_value(0, 0, "move_value_to")?;

		let other_cell = self.load(offset, "move_value_to")?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(current_value, factor, "move_value_to_mul")
		}?;

		let added = self
			.builder
			.build_int_add(other_cell, value_to_add, "move_value_to_add")?;

		self.store(added, offset, "move_value_to")
	}

	pub fn take_value_to(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_value = self.load(0, "take_value_to")?;
		self.store_value(0, 0, "take_value_to")?;

		self.move_pointer(offset)?;

		let other_cell = self.load(0, "take_value_to")?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(current_value, factor, "take_value_to_mul")
		}?;

		let added = self
			.builder
			.build_int_add(other_cell, value_to_add, "take_value_to_add")?;

		self.store(added, 0, "take_value_to")
	}

	pub fn fetch_value_from(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let other_cell= self.load(offset, "fetch_value_from")?;

		self.store_value(0, offset, "fetch_value_from")?;

		let current_cell = self.load(0, "fetch_value_from")?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(other_cell, factor, "fetch_value_from_mul")
		}?;

		let added =
			self.builder
				.build_int_add(current_cell, value_to_add, "fetch_value_from_add")?;

		self.store(added, 0, "fetch_value_from")
	}

	pub fn replace_value_from(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let other_cell= self.load(offset, "replace_value_from")?;

		self.store_value(0, offset, "replace_value_from")?;

		let value_to_store = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(other_cell, factor, "replace_value_from_mul")
		}?;

		self.store(value_to_store, 0, "replace_value_from")
	}

	pub fn scale_value(&self, factor: u8) -> Result<(), LlvmAssemblyError> {
		let cell = self.load(0, "scale_value")?;

		let value_to_store = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder.build_int_mul(cell, factor, "scale_value_mul")
		}?;

		self.store(value_to_store, 0, "scale_value")
	}
}
