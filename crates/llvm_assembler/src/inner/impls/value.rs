use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn move_value_to(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_value = self.take(0, "move_value_to")?;

		let (other_cell, gep) = self.load_from(offset, "move_value_to")?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(current_value, factor, "move_value_to_mul")?
		};

		let added = self
			.builder
			.build_int_add(other_cell, value_to_add, "move_value_to_add")?;

		self.store_into(added, gep)
	}

	pub fn take_value_to(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_value = self.take(0, "take_value_to")?;

		self.move_pointer(offset)?;

		let (other_cell, gep) = self.load_from(0, "take_value_to")?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(current_value, factor, "take_value_to_mul")
		}?;

		let added = self
			.builder
			.build_int_add(other_cell, value_to_add, "take_value_to_add")?;

		self.store_into(added, gep)
	}

	pub fn fetch_value_from(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let other_cell = self.take(offset, "fetch_value_from")?;

		let (current_cell, gep) = self.load_from(0, "fetch_value_from")?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(other_cell, factor, "fetch_value_from_mul")
		}?;

		let added =
			self.builder
				.build_int_add(current_cell, value_to_add, "fetch_value_from_add")?;

		self.store_into(added, gep)
	}

	pub fn replace_value_from(&self, factor: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		if matches!(factor, 1) {
			return self.replace_value_from_memmove(offset)
		}

		let other_cell = self.take(offset, "replace_value_from")?;

		let value_to_store = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(other_cell, factor, "replace_value_from_mul")
		}?;

		self.store(value_to_store, 0, "replace_value_from")
	}

	fn replace_value_from_memmove(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		// let current_offset = self.offset_ptr(offset)?;

		// let gep = self.gep(i8_type, offset, "replace_value_from_memmove")?;

		let current_cell_gep = {
			let ptr = self.offset_ptr(0)?;

			self.gep(i8_type, ptr, "replace_value_from_memmove")?
		};

		let other_value_gep = {
			let ptr = self.offset_ptr(offset)?;

			self.gep(i8_type, ptr, "replace_value_from_memmove")?
		};

		let one_value = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(1, false)
		};

		self.builder.build_memmove(current_cell_gep, 1, other_value_gep, 1, one_value)?;

		self.store_value_into(0, other_value_gep)
	}

	pub fn scale_value(&self, factor: u8) -> Result<(), LlvmAssemblyError> {
		let (cell, gep) = self.load_from(0, "scale_value")?;

		let value_to_store = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder.build_int_mul(cell, factor, "scale_value_mul")
		}?;

		self.store_into(value_to_store, gep)
	}
}
