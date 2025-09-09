use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn set_cell(&self, value: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let value = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(value.into(), false)
		};

		self.store(value, offset)
	}

	pub fn change_cell(&self, value: i8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_cell_value = self.load(offset)?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(value as u64, false)
		};

		let added =
			self.builder
				.build_int_add(current_cell_value, value_to_add, "change_cell_add")?;

		self.store(added, offset)
	}

	pub fn sub_cell(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let subtractor = self.load(0)?;

		self.set_cell(0, 0)?;

		let other_value = self.load(offset)?;

		let value_to_store = self
			.builder
			.build_int_sub(other_value, subtractor, "sub_cell_sub")?;

		self.store(value_to_store, offset)
	}

	pub fn duplicate_cell(&self, indices: &[i32]) -> Result<(), LlvmAssemblyError> {
		let value = self.load(0)?;

		self.set_cell(0, 0)?;

		for index in indices.iter().copied() {
			let other_value = self.load(index)?;

			let added_together =
				self.builder
					.build_int_add(other_value, value, "duplicate_cell_add")?;

			self.store(added_together, index)?;
		}

		Ok(())
	}
}
