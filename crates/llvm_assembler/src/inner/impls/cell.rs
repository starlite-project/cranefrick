use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn set_cell(&self, value: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		self.store_value(value, offset, "set_cell")
	}

	pub fn change_cell(&self, value: i8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_cell_value = self.load(offset, "change_cell")?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(value as u64, false)
		};

		let added =
			self.builder
				.build_int_add(current_cell_value, value_to_add, "change_cell_add")?;

		self.store(added, offset, "change_cell")
	}

	pub fn sub_cell(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let subtractor= self.load(0, "sub_cell")?;

		self.store_value(0, 0, "sub_cell")?;

		let other_value = self.load(offset, "sub_cell")?;

		let value_to_store = self
			.builder
			.build_int_sub(other_value, subtractor, "sub_cell_sub")?;

		self.store(value_to_store, offset, "sub_cell")
	}

	pub fn duplicate_cell(&self, indices: &[i32]) -> Result<(), LlvmAssemblyError> {
		let value = self.load(0, "duplicate_cell")?;

		self.store_value(0, 0, "duplicate_cell")?;

		for index in indices.iter().copied() {
			let other_value = self.load(index, "duplicate_cell")?;

			let added_together =
				self.builder
					.build_int_add(other_value, value, "duplicate_cell_add")?;

			self.store(added_together, index, "duplicate_cell")?;
		}

		Ok(())
	}
}
