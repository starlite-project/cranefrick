use frick_ir::{
	ChangeCellOptions, Factor, FactoredChangeCellOptions, SetManyCellsOptions, SetRangeOptions,
	ValuedChangeCellOptions, get_range, is_range,
};

use crate::{AssemblyError, ContextGetter as _, inner::InnerAssembler};

impl InnerAssembler<'_> {
	#[tracing::instrument(skip_all)]
	pub fn set_cell(&self, options: ValuedChangeCellOptions<u8>) -> Result<(), AssemblyError> {
		self.store_value(options.value(), options.offset(), "set_cell")
	}

	#[tracing::instrument(skip_all)]
	pub fn change_cell(&self, options: ValuedChangeCellOptions<i8>) -> Result<(), AssemblyError> {
		let (current_cell_value, gep) = self.load_from(options.offset(), "change_cell")?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(options.value() as u64, false)
		};

		let added =
			self.builder
				.build_int_add(current_cell_value, value_to_add, "change_cell_add\0")?;

		self.store_into(added, gep)?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub fn sub_cell_at(&self, options: FactoredChangeCellOptions<u8>) -> Result<(), AssemblyError> {
		let subtractor = {
			let i8_type = self.context().i8_type();

			let current_cell = self.take(0, "sub_cell_at")?;

			let factor_value = i8_type.const_int(options.factor().into(), false);

			self.builder
				.build_int_mul(current_cell, factor_value, "sub_cell_at_mul\0")?
		};

		let (other_value, gep) = self.load_from(options.offset(), "sub_cell_at")?;

		let value_to_store =
			self.builder
				.build_int_sub(other_value, subtractor, "sub_cell_at_sub\0")?;

		self.store_into(value_to_store, gep)?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub fn sub_from_cell(
		&self,
		options: FactoredChangeCellOptions<u8>,
	) -> Result<(), AssemblyError> {
		let subtractor = {
			let i8_type = self.context().i8_type();

			let current_cell = self.take(options.offset(), "sub_from_cell")?;

			let factor_value = i8_type.const_int(options.factor().into(), false);

			self.builder
				.build_int_mul(current_cell, factor_value, "sub_from_cell_mul\0")?
		};

		let (other_value, gep) = self.load_from(0, "sub_from_cell")?;

		let value_to_store =
			self.builder
				.build_int_sub(other_value, subtractor, "sub_from_cell_sub\0")?;

		self.store_into(value_to_store, gep)?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub fn duplicate_cell(
		&self,
		values: &[FactoredChangeCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		if is_vectorizable(values) {
			tracing::debug!("vectorizing duplicate_cell");
			self.duplicate_cell_vectorized(values)
		} else {
			tracing::debug!("unable to vectorize duplicate_cell");
			self.duplicate_cell_iterated(values)
		}
	}

	fn duplicate_cell_iterated(
		&self,
		values: &[FactoredChangeCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		let i8_type = self.context().i8_type();

		let (value, value_gep) = self.load_from(0, "duplicate_cell_iterated")?;

		for (factor, index) in values
			.iter()
			.copied()
			.map(FactoredChangeCellOptions::into_parts)
		{
			let (other_value, other_value_gep) =
				self.load_from(index, "duplicate_cell_iterated")?;

			let span = tracing::debug_span!(
				"duplicate_cell_iterated",
				index,
				factor = tracing::field::Empty
			);
			let _guard = span.enter();

			let modified_value = match factor {
				0 => {
					tracing::debug!("skipping cell");
					continue;
				}
				1 => {
					tracing::debug!("adding value directly");
					self.builder.build_int_add(
						other_value,
						value,
						"duplicate_cell_iterated_add\0",
					)?
				}
				x => {
					tracing::debug!("factoring value by {factor}, then adding");
					let factor = i8_type.const_int(x as u64, false);

					let factored_value = self.builder.build_int_mul(
						value,
						factor,
						"duplicate_cell_iterated_mul\0",
					)?;

					self.builder.build_int_add(
						other_value,
						factored_value,
						"duplicate_cell_iterated_add\0",
					)?
				}
			};

			self.store_into(modified_value, other_value_gep)?;
		}

		self.store_value_into(0, value_gep)?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	fn duplicate_cell_vectorized(
		&self,
		values: &[FactoredChangeCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let i8_vector_type = i8_type.vec_type(values.len() as u32);
		let i32_vector_type = i32_type.vec_type(values.len() as u32);

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
			.build_load(i8_vector_type, gep, "duplicate_cell_vectorized_load\0")?
			.into_vector_value();

		let vector_of_current_cells = {
			let tmp = self.builder.build_insert_element(
				undef,
				current_cell_value,
				zero_index,
				"duplicate_cell_vectorized_insertelement\0",
			)?;

			if matches!(values.len(), 2) {
				let one_index = i64_type.const_int(1, false);

				self.builder.build_insert_element(
					tmp,
					current_cell_value,
					one_index,
					"duplicate_cell_vectorized_insertelement\0",
				)?
			} else {
				self.builder.build_shuffle_vector(
					tmp,
					undef,
					i32_vector_type.const_zero(),
					"duplicate_cell_vectorized_shufflevector\0",
				)?
			}
		};

		let vector_of_new_values = {
			let mut vec = i8_vector_type.const_zero();

			for (i, factor) in values
				.iter()
				.copied()
				.map(FactoredChangeCellOptions::factor)
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
			.map(FactoredChangeCellOptions::factor)
			.all(|x| matches!(x, 1))
		{
			self.builder.build_int_add(
				loaded_values,
				vector_of_current_cells,
				"duplicate_cell_vectorized_add\0",
			)?
		} else {
			let multiplied = self.builder.build_int_mul(
				vector_of_current_cells,
				vector_of_new_values,
				"duplicate_cell_vectorized_mul\0",
			)?;

			self.builder.build_int_add(
				loaded_values,
				multiplied,
				"duplicate_cell_vectorized_add\0",
			)?
		};

		self.builder.build_store(gep, modified_vector_of_values)?;

		self.store_value_into(0, current_cell_gep)?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub fn set_many_cells(&self, options: &SetManyCellsOptions) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();

		let values_value = options
			.values()
			.iter()
			.copied()
			.map(|x| i8_type.const_int(x.into(), false))
			.collect::<Vec<_>>();

		let array_value = i8_type.const_array(&values_value);

		self.store(array_value, options.start(), "set_many_cells")
	}

	#[tracing::instrument(skip_all)]
	pub fn set_range(&self, options: SetRangeOptions) -> Result<(), AssemblyError> {
		let start = *options.range().start();
		let range_len = options.range().count();
		let i8_type = self.context().i8_type();

		let range_len_value = {
			let ptr_int_type = self.ptr_int_type;

			ptr_int_type.const_int(range_len as u64, false)
		};

		let value_value = i8_type.const_int(options.value().into(), false);

		let gep = self.tape_gep(i8_type, start, "set_range")?;

		self.builder
			.build_memset(gep, 16, value_value, range_len_value)?;

		Ok(())
	}
}

fn is_vectorizable(values: &[ChangeCellOptions<i8, Factor>]) -> bool {
	if !is_vector_size(values) {
		return false;
	}

	is_range(values) && values.len().is_power_of_two()
}

const fn is_vector_size<T>(values: &[T]) -> bool {
	values.len() >= 2
}
