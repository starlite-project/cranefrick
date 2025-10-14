use frick_ir::{ChangeCellOptions, Factor};
use inkwell::values::{IntValue, PointerValue};

use crate::{AssemblyError, ContextGetter as _, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip_all)]
	pub fn move_value_to(
		&self,
		options: ChangeCellOptions<u8, Factor>,
	) -> Result<(), AssemblyError> {
		let current_value = self.take(0, "move_value_to")?;

		let (other_cell, gep) = self.load_from(options.offset(), "move_value_to")?;

		self.duplicate_value_to(current_value, other_cell, options.factor(), gep)
	}

	#[tracing::instrument(skip_all)]
	pub fn copy_value_to(
		&self,
		options: ChangeCellOptions<u8, Factor>,
	) -> Result<(), AssemblyError> {
		let current_value = self.load(0, "copy_value_to")?;

		let (other_cell, gep) = self.load_from(options.offset(), "copy_value_to")?;

		self.duplicate_value_to(current_value, other_cell, options.factor(), gep)
	}

	#[tracing::instrument(skip_all)]
	fn duplicate_value_to(
		&self,
		current_value: IntValue<'ctx>,
		other_value: IntValue<'ctx>,
		factor: u8,
		gep: PointerValue<'ctx>,
	) -> Result<(), AssemblyError> {
		let value_to_add = if matches!(factor, 1) {
			current_value
		} else {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(current_value, factor, "duplicate_value_to_mul\0")?
		};

		let added =
			self.builder
				.build_int_add(other_value, value_to_add, "duplicate_value_to_add\0")?;

		self.store_into(added, gep)?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub fn take_value_to(
		&self,
		options: ChangeCellOptions<u8, Factor>,
	) -> Result<(), AssemblyError> {
		let current_value = self.take(0, "take_value_to")?;

		self.move_pointer(options.offset())?;

		let (other_cell, gep) = self.load_from(0, "take_value_to")?;

		let factor = options.factor();
		let value_to_add = if matches!(factor, 1) {
			current_value
		} else {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(current_value, factor, "take_value_to_mul\0")?
		};

		let added = self
			.builder
			.build_int_add(other_cell, value_to_add, "take_value_to_add\0")?;

		self.store_into(added, gep)?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub fn fetch_value_from(
		&self,
		options: ChangeCellOptions<u8, Factor>,
	) -> Result<(), AssemblyError> {
		let other_cell = self.take(options.offset(), "fetch_value_from")?;

		let (current_cell, gep) = self.load_from(0, "fetch_value_from")?;

		let factor = options.factor();
		let value_to_add = if matches!(factor, 1) {
			other_cell
		} else {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(other_cell, factor, "fetch_value_from_mul\0")?
		};

		let added =
			self.builder
				.build_int_add(current_cell, value_to_add, "fetch_value_from_add\0")?;

		self.store_into(added, gep)?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub fn replace_value_from(
		&self,
		options: ChangeCellOptions<u8, Factor>,
	) -> Result<(), AssemblyError> {
		if matches!(options.factor(), 1) {
			self.replace_value_from_memcpyed(options.offset())
		} else {
			self.replace_value_from_factorized(options)
		}
	}

	#[tracing::instrument(skip_all)]
	fn replace_value_from_memcpyed(&self, offset: i32) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i8_size = {
			let i64_type = context.i64_type();

			i64_type.const_int(1, false)
		};

		let current_cell_gep = self.tape_gep(i8_type, 0, "replace_value_from_memcpyed")?;

		let other_value_gep = self.tape_gep(i8_type, offset, "replace_value_from_memcpyed")?;

		self.builder
			.build_memcpy(current_cell_gep, 1, other_value_gep, 1, i8_size)?;

		self.store_value_into(0, other_value_gep)?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	fn replace_value_from_factorized(
		&self,
		options: ChangeCellOptions<u8, Factor>,
	) -> Result<(), AssemblyError> {
		let other_cell = self.take(options.offset(), "replace_value_from_factorized")?;

		let value_to_store = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int((options.factor()).into(), false);

			self.builder
				.build_int_mul(other_cell, factor, "replace_value_from_factorized_mul\0")?
		};

		self.store(value_to_store, 0, "replace_value_from_factorized")
	}

	#[tracing::instrument(skip_all)]
	pub fn scale_value(&self, factor: u8) -> Result<(), AssemblyError> {
		let (cell, gep) = self.load_from(0, "scale_value")?;

		let value_to_store = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(cell, factor, "scale_value_mul\0")
		}?;

		self.store_into(value_to_store, gep)?;

		Ok(())
	}
}
