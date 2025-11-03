use frick_ir::{
	ChangeManyCellsOptions, FactoredOffsetCellOptions, SetManyCellsOptions, SetRangeOptions,
	ValuedOffsetCellOptions,
};
use frick_utils::Convert as _;
use inkwell::{
	types::{IntType, VectorType},
	values::VectorValue,
};

use crate::{
	AssemblyError, BuilderExt as _, ContextGetter as _,
	inner::{InnerAssembler, utils::is_contiguous},
};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip(self))]
	pub fn set_cell(&self, options: ValuedOffsetCellOptions<u8>) -> Result<(), AssemblyError> {
		self.store_value_into_cell(options.value(), options.offset())
	}

	#[tracing::instrument(skip(self))]
	pub fn change_cell(&self, options: ValuedOffsetCellOptions<i8>) -> Result<(), AssemblyError> {
		let (current_cell_value, gep) = self.load_cell_and_pointer(options.offset())?;

		let value_to_add = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(options.value() as u64, false)
		};

		let added = self.builder.build_int_nsw_add(
			current_cell_value,
			value_to_add,
			"change_cell_add\0",
		)?;

		self.store_into(added, gep)
	}

	#[tracing::instrument(skip(self))]
	pub fn sub_cell_at(&self, options: FactoredOffsetCellOptions<u8>) -> Result<(), AssemblyError> {
		let subtractor = {
			let i8_type = self.context().i8_type();

			let current_cell = self.take(0)?;

			let factor_value = i8_type.const_int(options.factor().convert::<u64>(), false);

			self.builder
				.build_int_mul(current_cell, factor_value, "sub_cell_at_mul\0")?
		};

		let (other_value, gep) = self.load_cell_and_pointer(options.offset())?;

		let value_to_store =
			self.builder
				.build_int_sub(other_value, subtractor, "sub_cell_at_sub\0")?;

		self.store_into(value_to_store, gep)
	}

	#[tracing::instrument(skip(self))]
	pub fn sub_from_cell(
		&self,
		options: FactoredOffsetCellOptions<u8>,
	) -> Result<(), AssemblyError> {
		let subtractor = {
			let i8_type = self.context().i8_type();

			let current_cell = self.take(options.offset())?;

			let factor_value = i8_type.const_int(options.factor().convert::<u64>(), false);

			self.builder
				.build_int_mul(current_cell, factor_value, "sub_from_cell_mul\0")?
		};

		let (other_value, gep) = self.load_cell_and_pointer(0)?;

		let value_to_store =
			self.builder
				.build_int_sub(other_value, subtractor, "sub_from_cell_sub\0")?;

		self.store_into(value_to_store, gep)
	}

	#[tracing::instrument(skip(self))]
	pub fn duplicate_cell(
		&self,
		values: &[FactoredOffsetCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let i8_vec_type = i8_type.vec_type(values.len() as u32);
		let i32_vec_type = i32_type.vec_type(values.len() as u32);

		let current_cell = self.take(0)?;

		let i64_zero = i64_type.const_zero();

		let tmp = self.builder.build_insert_element(
			i8_vec_type.get_poison(),
			current_cell,
			i64_zero,
			"duplicate_cell_insert_element\0",
		)?;

		let vec_of_current_cell = self.builder.build_shuffle_vector(
			tmp,
			i8_vec_type.get_poison(),
			i32_vec_type.const_zero(),
			"duplicate_cell_shuffle_vector\0",
		)?;

		if is_contiguous(values) {
			self.duplicate_cell_contiguous(values, vec_of_current_cell, i8_type, i8_vec_type)
		} else {
			self.duplicate_cell_scattered(values, vec_of_current_cell, i8_type, i8_vec_type)
		}
	}

	fn duplicate_cell_scattered(
		&self,
		values: &[FactoredOffsetCellOptions<i8>],
		vec_of_current_cell: VectorValue<'ctx>,
		i8_type: IntType<'ctx>,
		i8_vec_type: VectorType<'ctx>,
	) -> Result<(), AssemblyError> {
		let vec_of_indices = {
			let offsets = values.iter().map(|x| x.offset()).collect::<Vec<_>>();

			self.offset_many_pointers(&offsets)?
		};

		let vec_of_pointers = unsafe {
			self.builder.build_vec_gep(
				i8_type,
				self.pointers.tape,
				vec_of_indices,
				"duplicate_cell_scattered_gep\0",
			)?
		};

		let vec_of_loaded_values = self.call_vector_gather(i8_vec_type, vec_of_pointers)?;

		let vec_of_modified_values = if values.iter().all(|x| matches!(x.offset(), 1)) {
			self.builder.build_int_nsw_add(
				vec_of_current_cell,
				vec_of_loaded_values,
				"duplicate_cell_scattered_vector_add\0",
			)?
		} else {
			let vec_of_factors = {
				let vec_of_values = values
					.iter()
					.map(|v| i8_type.const_int(v.factor() as u64, false))
					.collect::<Vec<_>>();

				VectorType::const_vector(&vec_of_values)
			};

			let vec_of_scaled_current_cell = self.builder.build_int_mul(
				vec_of_current_cell,
				vec_of_factors,
				"duplicate_cell_scattered_vector_mul\0",
			)?;

			self.builder.build_int_nsw_add(
				vec_of_loaded_values,
				vec_of_scaled_current_cell,
				"duplicate_cell_scattered_vector_add\0",
			)?
		};

		self.call_vector_scatter(vec_of_modified_values, vec_of_pointers)?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	fn duplicate_cell_contiguous(
		&self,
		values: &[FactoredOffsetCellOptions<i8>],
		vec_of_current_cell: VectorValue<'ctx>,
		i8_type: IntType<'ctx>,
		i8_vec_type: VectorType<'ctx>,
	) -> Result<(), AssemblyError> {
		let start_of_range = values.iter().map(|x| x.offset()).min().unwrap();

		let gep = self.tape_gep(i8_vec_type, start_of_range)?;

		let vec_of_loaded_values = self.load_from(i8_vec_type, gep)?;

		let vec_of_modified_values = if values.iter().all(|x| matches!(x.offset(), 1)) {
			self.builder.build_int_nsw_add(
				vec_of_current_cell,
				vec_of_loaded_values,
				"duplicate_cell_contiguous_vector_add\0",
			)?
		} else {
			let vec_of_factors = {
				let vec_of_values = values
					.iter()
					.map(|v| i8_type.const_int(v.factor() as u64, false))
					.collect::<Vec<_>>();

				VectorType::const_vector(&vec_of_values)
			};

			let vec_of_scaled_current_cell = self.builder.build_int_mul(
				vec_of_current_cell,
				vec_of_factors,
				"duplicate_cell_contiguous_vector_mul\0",
			)?;

			self.builder.build_int_nsw_add(
				vec_of_loaded_values,
				vec_of_scaled_current_cell,
				"duplicate_cell_contiguous_vector_add\0",
			)?
		};

		self.store_into(vec_of_modified_values, gep)
	}

	#[tracing::instrument(skip(self))]
	pub fn set_many_cells(&self, options: &SetManyCellsOptions) -> Result<(), AssemblyError> {
		let i8_type = self.context().i8_type();

		let values_to_store = options
			.values()
			.iter()
			.map(|x| i8_type.const_int((*x).convert::<u64>(), false))
			.collect::<Vec<_>>();

		let vec_to_store = VectorType::const_vector(&values_to_store);

		self.store_into_cell(vec_to_store, options.start())
	}

	#[tracing::instrument(skip(self))]
	pub fn set_range(&self, options: SetRangeOptions) -> Result<(), AssemblyError> {
		let start = *options.range().start();
		let range_len = options.range().count();
		let i8_type = self.context().i8_type();

		let range_len_value = {
			let ptr_int_type = self.ptr_int_type;

			ptr_int_type.const_int(range_len as u64, false)
		};

		let value_value = i8_type.const_int(options.value().convert::<u64>(), false);

		let gep = self.tape_gep(i8_type, start)?;

		self.builder
			.build_memset(gep, 1, value_value, range_len_value)?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub fn change_many_cells(&self, options: &ChangeManyCellsOptions) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i8_vector_type = i8_type.vec_type(options.values().len() as u32);

		let gep = self.tape_gep(i8_vector_type, options.start())?;

		let vec_of_tape_values = self.load_from(i8_vector_type, gep)?;

		let vec_of_change_values = {
			let vec_of_values = options
				.values()
				.iter()
				.map(|x| i8_type.const_int(*x as u64, false))
				.collect::<Vec<_>>();

			VectorType::const_vector(&vec_of_values)
		};

		let vec_of_values_to_store = self.builder.build_int_nsw_add(
			vec_of_tape_values,
			vec_of_change_values,
			"change_many_cells_vector_add",
		)?;

		self.store_into(vec_of_values_to_store, gep)
	}
}
