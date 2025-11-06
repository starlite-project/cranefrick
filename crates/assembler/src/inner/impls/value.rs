use frick_ir::{Factor, OffsetCellOptions};
use frick_utils::Convert as _;

use crate::{AssemblyError, ContextGetter as _, inner::InnerAssembler};

impl InnerAssembler<'_> {
	#[tracing::instrument(skip(self))]
	pub fn move_value_to(
		&self,
		options: OffsetCellOptions<u8, Factor>,
	) -> Result<(), AssemblyError> {
		let current_value = self.take(0)?;

		let (other_cell, gep) = self.load_cell_and_pointer(options.offset())?;

		let value_to_add = if matches!(options.factor(), 1) {
			current_value
		} else {
			let factor = {
				let i8_type = self.context().i8_type();

				i8_type.const_int(options.factor().convert::<u64>(), false)
			};

			self.builder
				.build_int_mul(current_value, factor, "move_value_to_mul\0")?
		};

		let added_together =
			self.builder
				.build_int_add(other_cell, value_to_add, "move_value_to_add\0")?;

		self.store_into(added_together, gep)?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub fn take_value_to(
		&self,
		options: OffsetCellOptions<u8, Factor>,
	) -> Result<(), AssemblyError> {
		let current_value = self.take(0)?;

		self.move_pointer(options.offset())?;

		let (other_cell, gep) = self.load_cell_and_pointer(0)?;

		let factor = options.factor();
		let value_to_add = if matches!(factor, 1) {
			current_value
		} else {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.convert::<u64>(), false);

			self.builder
				.build_int_mul(current_value, factor, "take_value_to_mul\0")?
		};

		let added = self
			.builder
			.build_int_add(other_cell, value_to_add, "take_value_to_add\0")?;

		self.store_into(added, gep)?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub fn fetch_value_from(
		&self,
		options: OffsetCellOptions<u8, Factor>,
	) -> Result<(), AssemblyError> {
		let other_cell = self.take(options.offset())?;

		let (current_cell, gep) = self.load_cell_and_pointer(0)?;

		let factor = options.factor();
		let value_to_add = if matches!(factor, 1) {
			other_cell
		} else {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.convert::<u64>(), false);

			self.builder
				.build_int_mul(other_cell, factor, "fetch_value_from_mul\0")?
		};

		let added =
			self.builder
				.build_int_add(current_cell, value_to_add, "fetch_value_from_add\0")?;

		self.store_into(added, gep)?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub fn replace_value_from(
		&self,
		options: OffsetCellOptions<u8, Factor>,
	) -> Result<(), AssemblyError> {
		if matches!(options.factor(), 1) {
			self.replace_value_from_memmoved(options.offset())
		} else {
			self.replace_value_from_factorized(options)
		}
	}

	#[tracing::instrument(skip(self))]
	fn replace_value_from_memmoved(&self, offset: i32) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i8_size = {
			let i64_type = context.i64_type();

			i64_type.const_int(1, false)
		};

		let current_cell_gep = self.tape_gep(i8_type, 0)?;

		let other_value_gep = self.tape_gep(i8_type, offset)?;

		self.builder
			.build_memmove(current_cell_gep, 1, other_value_gep, 1, i8_size)?;

		self.store_value_into(0, other_value_gep)?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	fn replace_value_from_factorized(
		&self,
		options: OffsetCellOptions<u8, Factor>,
	) -> Result<(), AssemblyError> {
		let other_cell = self.take(options.offset())?;

		let value_to_store = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int((options.factor()).convert::<u64>(), false);

			self.builder
				.build_int_mul(other_cell, factor, "replace_value_from_factorized_mul\0")?
		};

		self.store_into_cell(value_to_store, 0)?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub fn scale_value(&self, factor: u8) -> Result<(), AssemblyError> {
		let (cell, gep) = self.load_cell_and_pointer(0)?;

		let value_to_store = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.convert::<u64>(), false);

			self.builder
				.build_int_mul(cell, factor, "scale_value_mul\0")
		}?;

		self.store_into(value_to_store, gep)?;

		Ok(())
	}
}
