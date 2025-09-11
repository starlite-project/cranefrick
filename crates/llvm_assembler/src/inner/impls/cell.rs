use std::ops::RangeInclusive;

use frick_ir::DuplicateCellData;
use inkwell::types::VectorType;

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn set_cell(&self, value: u8, offset: i32) -> Result<(), LlvmAssemblyError> {
		self.store_value(value, offset, "set_cell")
	}

	pub fn change_cell(&self, value: i8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let (current_cell_value, gep) = self.load_from(offset, "change_cell")?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(value as u64, false)
		};

		let added =
			self.builder
				.build_int_add(current_cell_value, value_to_add, "change_cell_add")?;

		self.store_into(added, gep)
	}

	pub fn sub_cell(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let subtractor = self.take(0, "sub_cell")?;

		let (other_value, gep) = self.load_from(offset, "sub_cell")?;

		let value_to_store = self
			.builder
			.build_int_sub(other_value, subtractor, "sub_cell_sub")?;

		self.store_into(value_to_store, gep)
	}

	pub fn duplicate_cell(&self, values: &[DuplicateCellData]) -> Result<(), LlvmAssemblyError> {
		if is_range(values) {
			self.duplicate_cell_vectorized(values)
		} else {
			self.duplicate_cell_iterated(values)
		}
	}

	fn duplicate_cell_iterated(
		&self,
		values: &[DuplicateCellData],
	) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		let value = self.take(0, "duplicate_cell_iterated")?;

		for (factor, index) in values.iter().copied().map(DuplicateCellData::into_parts) {
			let (other_value, gep) = self.load_from(index, "duplicate_cell_iterated")?;

			let factor_value = i8_type.const_int(factor as u64, false);

			let factored_value =
				self.builder
					.build_int_mul(value, factor_value, "duplicate_cell_iterated_mul")?;

			let modified_other_value = self.builder.build_int_add(
				other_value,
				factored_value,
				"duplicate_cell_iterated_add",
			)?;

			self.store_into(modified_other_value, gep)?;
		}

		Ok(())
	}

	fn duplicate_cell_vectorized(
		&self,
		values: &[DuplicateCellData],
	) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();
		let i64_type = self.context().i64_type();
		let i8_vector_type = i8_type.vec_type(values.len() as u32);

		let current_cell_value = self.take(0, "duplicate_cell_vectorized")?;

		let zero_index = i64_type.const_zero();
		let undef = i8_vector_type.get_undef();

		let range_start = {
			let range = get_range(values).unwrap();

			*range.start()
		};

		let current_offset = self.offset_ptr(range_start)?;

		let gep = self.gep(i8_type, current_offset, "duplicate_cell_vectorized")?;

		let loaded_values = self
			.builder
			.build_load(i8_vector_type, gep, "duplicate_cell_vectorized_load")?
			.into_vector_value();

		let vector_of_current_cells = {
			let tmp = self.builder.build_insert_element(
				undef,
				current_cell_value,
				zero_index,
				"duplicate_cell_vectorized_insertelement",
			)?;

			self.builder.build_shuffle_vector(
				tmp,
				undef,
				i8_vector_type.const_zero(),
				"duplicate_cell_vectorized_shufflevector",
			)?
		};

		let vector_of_new_values = {
			let mut vec_of_values_for_vector = Vec::with_capacity(values.len());

			for factor in values.iter().copied().map(DuplicateCellData::factor) {
				let factor = i8_type.const_int(factor as u64, false);

				vec_of_values_for_vector.push(factor);
			}

			VectorType::const_vector(&vec_of_values_for_vector)
		};

		let multiplied = self.builder.build_int_mul(
			vector_of_current_cells,
			vector_of_new_values,
			"duplicate_cell_vectorized_mul",
		)?;

		let modified_vector_of_values = self.builder.build_int_add(
			multiplied,
			loaded_values,
			"duplicate_cell_vectorized_add",
		)?;

		self.builder.build_store(gep, modified_vector_of_values)?;

		Ok(())
	}
}

fn is_range(values: &[DuplicateCellData]) -> bool {
	if values.len() <= 1 {
		return false;
	}

	let Some(range) = get_range(values) else {
		return false;
	};

	for offset in values.iter().copied().map(DuplicateCellData::offset) {
		if !range.contains(&offset) {
			return false;
		}
	}

	if range.count() != values.len() {
		return false;
	}

	true
}

fn get_range(values: &[DuplicateCellData]) -> Option<RangeInclusive<i32>> {
	assert!(values.len() > 1);

	let first = values.first().copied()?;

	let last = values.last().copied()?;

	Some(first.offset()..=last.offset())
}
