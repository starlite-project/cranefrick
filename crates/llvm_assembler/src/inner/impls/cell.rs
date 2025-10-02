use std::ops::RangeInclusive;

use frick_ir::{CellChangeOptions, get_range, is_range};
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

	pub fn sub_cell_at(&self, options: CellChangeOptions) -> Result<(), LlvmAssemblyError> {
		let subtractor = {
			let i8_type = self.context().i8_type();

			let current_cell = self.take(0, "sub_cell_at")?;

			let factor_value = i8_type.const_int(options.value().into(), false);

			self.builder
				.build_int_mul(current_cell, factor_value, "sub_cell_at_mul")?
		};

		let (other_value, gep) = self.load_from(options.offset(), "sub_cell_at")?;

		let value_to_store =
			self.builder
				.build_int_sub(other_value, subtractor, "sub_cell_at_sub")?;

		self.store_into(value_to_store, gep)
	}

	pub fn sub_from_cell(&self, options: CellChangeOptions) -> Result<(), LlvmAssemblyError> {
		let subtractor = {
			let i8_type = self.context().i8_type();

			let current_cell = self.take(options.offset(), "sub_from_cell")?;

			let factor_value = i8_type.const_int(options.value().into(), false);

			self.builder
				.build_int_mul(current_cell, factor_value, "sub_from_cell_mul")?
		};

		let (other_value, gep) = self.load_from(0, "sub_from_cell")?;

		let value_to_store =
			self.builder
				.build_int_sub(other_value, subtractor, "sub_from_cell_sub")?;

		self.store_into(value_to_store, gep)
	}

	pub fn duplicate_cell(
		&self,
		values: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
		if is_vectorizable(values) {
			self.duplicate_cell_vectorized(values)
		} else {
			self.duplicate_cell_iterated(values)
		}
	}

	fn duplicate_cell_iterated(
		&self,
		values: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		let (value, value_gep) = self.load_from(0, "duplicate_cell_iterated")?;

		for (factor, index) in values.iter().copied().map(CellChangeOptions::into_parts) {
			let (other_value, other_value_gep) =
				self.load_from(index, "duplicate_cell_iterated")?;

			let modified_value = match factor {
				0 => {
					continue;
				}
				1 => {
					self.builder
						.build_int_add(other_value, value, "duplicate_cell_iterated_add")?
				}
				x => {
					let factor = i8_type.const_int(x as u64, false);

					let factored_value =
						self.builder
							.build_int_mul(value, factor, "duplicate_cell_iterated_mul")?;

					self.builder.build_int_add(
						other_value,
						factored_value,
						"duplicate_cell_iterated_add",
					)?
				}
			};

			self.store_into(modified_value, other_value_gep)?;
		}

		self.store_value_into(0, value_gep)
	}

	fn duplicate_cell_vectorized(
		&self,
		values: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i64_type = context.i64_type();
		let i8_vector_type = i8_type.vec_type(values.len() as u32);

		let (current_cell_value, current_cell_gep) =
			self.load_from(0, "duplicate_cell_vectorized")?;

		let zero_index = i64_type.const_zero();
		let undef = i8_vector_type.get_undef();

		let range_start = {
			let range = get_range(values).unwrap();

			*range.start()
		};

		let gep = self.tape_gep(i8_vector_type, range_start, "duplicate_cell_vectorized")?;

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
			let mut vec = i8_vector_type.const_zero();

			for (i, factor) in values
				.iter()
				.copied()
				.map(CellChangeOptions::value)
				.enumerate()
			{
				let index = i64_type.const_int(i as u64, false);

				let factor = i8_type.const_int(factor as u64, false);

				vec = vec.const_insert_element(index, factor).into_vector_value();
			}

			vec
		};

		let modified_vector_of_values = if values
			.iter()
			.copied()
			.map(CellChangeOptions::value)
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
		let context = self.context();

		let i8_type = context.i8_type();

		let values_values = values
			.iter()
			.copied()
			.map(|x| i8_type.const_int(x.into(), false))
			.collect::<Vec<_>>();

		let vector = VectorType::const_vector(&values_values);

		self.store(vector, start, "set_many_cells")
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

		let value_value = i8_type.const_int(value.into(), false);

		let gep = self.tape_gep(i8_type, start, "set_range")?;

		self.builder
			.build_memset(gep, 1, value_value, range_len_value)?;

		Ok(())
	}
}

fn is_vectorizable(values: &[CellChangeOptions<i8>]) -> bool {
	if !is_vector_size(values) {
		return false;
	}

	is_range(values) && values.len().is_power_of_two()
}

const fn is_vector_size<T>(values: &[T]) -> bool {
	values.len() >= 2
}
