use frick_ir::{
	FactoredChangeCellOptions, SetManyCellsOptions, SetRangeOptions, ValuedChangeCellOptions,
};
use inkwell::types::VectorType;

use crate::{AssemblyError, BuilderExt as _, ContextGetter as _, inner::InnerAssembler};

impl InnerAssembler<'_> {
	#[tracing::instrument(skip_all)]
	pub fn set_cell(&self, options: ValuedChangeCellOptions<u8>) -> Result<(), AssemblyError> {
		self.store_value_into_cell(options.value(), options.offset(), "set_cell")
	}

	#[tracing::instrument(skip_all)]
	pub fn change_cell(&self, options: ValuedChangeCellOptions<i8>) -> Result<(), AssemblyError> {
		let (current_cell_value, gep) =
			self.load_cell_and_pointer(options.offset(), "change_cell")?;

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

		let (other_value, gep) = self.load_cell_and_pointer(options.offset(), "sub_cell_at")?;

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

		let (other_value, gep) = self.load_cell_and_pointer(0, "sub_from_cell")?;

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
		let context = self.context();

		let ptr_int_type = self.ptr_int_type;
		let bool_type = context.bool_type();
		let i8_type = context.i8_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let i8_vec_type = i8_type.vec_type(values.len() as u32);
		let i32_vec_type = i32_type.vec_type(values.len() as u32);
		let ptr_int_vec_type = ptr_int_type.vec_type(values.len() as u32);

		let vec_of_current_cell = {
			let current_cell = self.take(0, "duplicate_cell")?;

			let zero_index = i64_type.const_zero();

			let tmp = self.builder.build_insert_element(
				i8_vec_type.get_undef(),
				current_cell,
				zero_index,
				"duplicate_cell_insert_element\0",
			)?;

			self.builder.build_shuffle_vector(
				tmp,
				i8_vec_type.get_undef(),
				i32_vec_type.const_zero(),
				"duplicate_cell_shuffle_vector",
			)?
		};

		let vec_of_indices = {
			let mut vec = ptr_int_vec_type.const_zero();

			for (i, offset) in values
				.iter()
				.copied()
				.map(FactoredChangeCellOptions::offset)
				.enumerate()
			{
				let offset = self.offset_pointer(offset, "duplicate_cell")?;

				let index = i64_type.const_int(i as u64, false);

				vec = self.builder.build_insert_element(
					vec,
					offset,
					index,
					"duplicate_cell_insert_element\0",
				)?;
			}

			vec
		};

		let vec_of_ptrs = unsafe {
			self.builder.build_vec_gep(
				i8_type,
				self.pointers.tape,
				vec_of_indices,
				"duplicate_cell_gep\0",
			)?
		};

		let vector_gather = self.get_vector_gather(i8_vec_type)?;

		let vec_load_store_alignment = i32_type.const_int(1, false);

		let bool_vec_all_on = {
			let vec_of_trues = vec![bool_type.const_all_ones(); values.len()];

			VectorType::const_vector(&vec_of_trues)
		};

		let vec_of_loaded_values = self
			.builder
			.build_call(
				vector_gather,
				&[
					vec_of_ptrs.into(),
					vec_load_store_alignment.into(),
					bool_vec_all_on.into(),
					i8_vec_type.get_undef().into(),
				],
				"duplicate_cell_vector_load_call\0",
			)?
			.try_as_basic_value()
			.unwrap_left()
			.into_vector_value();

		let vec_of_modified_values = if values
			.iter()
			.copied()
			.map(FactoredChangeCellOptions::factor)
			.all(|x| matches!(x, 1))
		{
			self.builder.build_int_add(
				vec_of_current_cell,
				vec_of_loaded_values,
				"duplicate_cell_add\0",
			)?
		} else {
			let vec_of_factors = {
				let mut vec = i8_vec_type.const_zero();

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

			let multiplied_vec = self.builder.build_int_mul(
				vec_of_current_cell,
				vec_of_factors,
				"duplicate_cell_mul",
			)?;

			self.builder.build_int_add(
				multiplied_vec,
				vec_of_loaded_values,
				"duplicate_cell_add\0",
			)?
		};

		let vec_scatter = self.get_vector_scatter(i8_vec_type)?;

		self.builder.build_call(
			vec_scatter,
			&[
				vec_of_modified_values.into(),
				vec_of_ptrs.into(),
				vec_load_store_alignment.into(),
				bool_vec_all_on.into(),
			],
			"duplicate_cell_store\0",
		)?;

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

		self.store_into_cell(array_value, options.start(), "set_many_cells")
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
			.build_memset(gep, 1, value_value, range_len_value)?;

		Ok(())
	}
}
