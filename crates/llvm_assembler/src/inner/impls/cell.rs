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
		if is_vectorizable(values) {
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

		let (value, value_gep) = self.load_from(0, "duplicate_cell_iterated")?;

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

		self.store_value_into(0, value_gep)
	}

	fn duplicate_cell_vectorized(
		&self,
		values: &[DuplicateCellData],
	) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();
		let i64_type = self.context().i64_type();
		let i8_vector_type = i8_type.vec_type(values.len() as u32);

		let (current_cell_value, current_cell_gep) =
			self.load_from(0, "duplicate_cell_vectorized")?;

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

		let modified_vector_of_values = if values
			.iter()
			.copied()
			.map(DuplicateCellData::factor)
			.all(|x| matches!(x, 1))
		{
			self.builder.build_int_add(
				loaded_values,
				vector_of_current_cells,
				"duplicate_cell_vectorized_add",
			)?
		} else {
			let multiplied = self.builder.build_int_mul(
				vector_of_current_cells,
				vector_of_new_values,
				"duplicate_cell_vectorized_add",
			)?;

			self.builder.build_int_add(
				loaded_values,
				multiplied,
				"duplicate_cell_vectorized_add",
			)?
		};

		self.builder.build_store(gep, modified_vector_of_values)?;
		self.store_value_into(0, current_cell_gep)
	}

	pub fn set_many_cells(&self, values: &[u8], start: i32) -> Result<(), LlvmAssemblyError> {
		if is_vector_size(values) {
			tracing::info!("made it, values = {values:?}");

			self.set_many_cells_vectorized(values, start)
		} else if values.len() <= 64 {
			self.set_many_cells_scratch(values, start)
		} else {
			self.set_many_cells_iterated(values, start)
		}
	}

	fn set_many_cells_vectorized(
		&self,
		values: &[u8],
		start: i32,
	) -> Result<(), LlvmAssemblyError> {
		assert!(is_vector_size(values));

		let i8_type = self.context().i8_type();

		let vector_of_values = {
			let vec_of_values = values
				.iter()
				.copied()
				.map(|v| i8_type.const_int(v.into(), false))
				.collect::<Vec<_>>();

			VectorType::const_vector(&vec_of_values)
		};

		let current_offset = self.offset_ptr(start)?;

		let gep = self.gep(i8_type, current_offset, "set_many_cells_vectorized")?;

		self.builder.build_store(gep, vector_of_values)?;

		Ok(())
	}

	fn set_many_cells_iterated(&self, values: &[u8], start: i32) -> Result<(), LlvmAssemblyError> {
		for (i, value) in values.iter().copied().enumerate() {
			self.store_value(
				value,
				start.wrapping_add_unsigned(i as u32),
				"set_many_cells_iterated",
			)?;
		}

		Ok(())
	}

	fn set_many_cells_scratch(&self, values: &[u8], start: i32) -> Result<(), LlvmAssemblyError> {
		assert!(
			values.len() <= 64,
			"too many values (this shouldn't happen)"
		);

		let i8_type = self.context().i8_type();
		let ptr_int_type = self.ptr_int_type;

		let array_len_value = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(values.len() as u64, false)
		};

		self.builder.build_call(
			self.functions.lifetime.start,
			&[array_len_value.into(), self.scratch_buffer.into()],
			"",
		)?;

		for (i, value) in values.iter().copied().enumerate().map(|(i, v)| {
			(
				ptr_int_type.const_int(i as u64, false),
				i8_type.const_int(v.into(), false),
			)
		}) {
			let gep = unsafe {
				self.builder.build_in_bounds_gep(
					i8_type,
					self.scratch_buffer,
					&[i],
					"set_many_cells_gep",
				)?
			};

			self.builder.build_store(gep, value)?;
		}

		let current_offset = self.offset_ptr(start)?;

		let gep = self.gep(i8_type, current_offset, "set_many_cells")?;

		self.builder
			.build_memcpy(gep, 1, self.scratch_buffer, 1, array_len_value)?;

		self.builder.build_call(
			self.functions.lifetime.end,
			&[array_len_value.into(), self.scratch_buffer.into()],
			"",
		)?;

		Ok(())
	}

	pub fn set_range(
		&self,
		value: u8,
		range: RangeInclusive<i32>,
	) -> Result<(), LlvmAssemblyError> {
		let start = *range.start();
		let range_len = range.count();
		let i8_type = self.context().i8_type();

		let range_len_value = {
			let ptr_int_type = self.ptr_int_type;

			ptr_int_type.const_int(range_len as u64, false)
		};

		let start_value = self.offset_ptr(start)?;

		let value_value = i8_type.const_int(value.into(), false);

		let gep = self.gep(i8_type, start_value, "mem_set")?;

		self.builder
			.build_memset(gep, 1, value_value, range_len_value)?;

		Ok(())
	}
}

fn is_vectorizable(values: &[DuplicateCellData]) -> bool {
	if !is_vector_size(values) {
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

	if range.count() == values.len() {
		values.len().is_power_of_two()
	} else {
		false
	}
}

fn get_range(values: &[DuplicateCellData]) -> Option<RangeInclusive<i32>> {
	assert!(values.len() > 1);

	let first = values.first().copied()?;

	let last = values.last().copied()?;

	Some(first.offset()..=last.offset())
}

const fn is_vector_size<T>(values: &[T]) -> bool {
	matches!(values.len(), 2 | 4 | 8 | 16 | 32 | 64)
}
