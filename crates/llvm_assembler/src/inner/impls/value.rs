use frick_ir::CellChangeOptions;
use inkwell::values::{IntValue, PointerValue};

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn move_value_to(&self, options: CellChangeOptions) -> Result<(), LlvmAssemblyError> {
		let current_value = self.take(0, "move_value_to")?;

		let (other_cell, gep) = self.load_from(options.offset(), "move_value_to")?;

		self.duplicate_value_to(current_value, other_cell, options.value(), gep)
	}

	pub fn copy_value_to(&self, options: CellChangeOptions) -> Result<(), LlvmAssemblyError> {
		let current_value = self.load(0, "copy_value_to")?;

		let (other_cell, gep) = self.load_from(options.offset(), "copy_value_to")?;

		self.duplicate_value_to(current_value, other_cell, options.value(), gep)
	}

	fn duplicate_value_to(
		&self,
		current_value: IntValue<'ctx>,
		other_value: IntValue<'ctx>,
		factor: u8,
		gep: PointerValue<'ctx>,
	) -> Result<(), LlvmAssemblyError> {
		let value_to_add = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int(factor.into(), false);

			self.builder
				.build_int_mul(current_value, factor, "duplicate_value_to_mul")?
		};

		let added =
			self.builder
				.build_int_add(other_value, value_to_add, "duplicate_value_to_add")?;

		self.store_into(added, gep)
	}

	pub fn take_value_to(&self, options: CellChangeOptions) -> Result<(), LlvmAssemblyError> {
		let current_value = self.take(0, "take_value_to")?;

		self.move_pointer(options.offset())?;

		let (other_cell, gep) = self.load_from(0, "take_value_to")?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int((options.value()).into(), false);

			self.builder
				.build_int_mul(current_value, factor, "take_value_to_mul")
		}?;

		let added = self
			.builder
			.build_int_add(other_cell, value_to_add, "take_value_to_add")?;

		self.store_into(added, gep)
	}

	pub fn fetch_value_from(&self, options: CellChangeOptions) -> Result<(), LlvmAssemblyError> {
		let other_cell = self.take(options.offset(), "fetch_value_from")?;

		let (current_cell, gep) = self.load_from(0, "fetch_value_from")?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int((options.value()).into(), false);

			self.builder
				.build_int_mul(other_cell, factor, "fetch_value_from_mul")
		}?;

		let added =
			self.builder
				.build_int_add(current_cell, value_to_add, "fetch_value_from_add")?;

		self.store_into(added, gep)
	}

	pub fn replace_value_from(&self, options: CellChangeOptions) -> Result<(), LlvmAssemblyError> {
		if matches!(options.value(), 1) {
			self.replace_value_from_memmoved(options.offset())
		} else {
			self.replace_value_from_factorized(options)
		}
	}

	fn replace_value_from_memmoved(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		let current_cell_gep = {
			let ptr = self.offset_pointer(0)?;

			self.gep(i8_type, ptr, "replace_value_from_memmoved")?
		};

		let other_value_gep = {
			let ptr = self.offset_pointer(offset)?;

			self.gep(i8_type, ptr, "replace_value_from_memmoved")?
		};

		let one_value = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(1, false)
		};

		self.builder
			.build_memmove(current_cell_gep, 1, other_value_gep, 1, one_value)?;

		self.store_value_into(0, other_value_gep)
	}

	fn replace_value_from_factorized(
		&self,
		options: CellChangeOptions,
	) -> Result<(), LlvmAssemblyError> {
		let other_cell = self.take(options.offset(), "replace_value_from_factorized")?;

		let value_to_store = {
			let i8_type = self.context().i8_type();

			let factor = i8_type.const_int((options.value()).into(), false);

			self.builder
				.build_int_mul(other_cell, factor, "replace_value_from_factorized_mul")?
		};

		self.store(value_to_store, 0, "replace_value_from_factorized")
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
