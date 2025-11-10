use frick_ir::{
	ChangeManyCellsOptions, FactoredOffsetCellOptions, SetManyCellsOptions, SetRangeOptions,
	ValuedOffsetCellOptions,
};
use frick_utils::Convert as _;
use inkwell::types::VectorType;

use crate::{AssemblyError, BuilderExt as _, ContextGetter as _, inner::InnerAssembler};

impl InnerAssembler<'_> {
	#[tracing::instrument(skip(self))]
	pub fn set_cell(&self, options: ValuedOffsetCellOptions<u8>) -> Result<(), AssemblyError> {
		self.store_value_into_cell(options.value(), options.offset())?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub fn change_cell(&self, options: ValuedOffsetCellOptions<i8>) -> Result<(), AssemblyError> {
		let (current_cell_value, gep) = self.load_cell_and_pointer(options.offset())?;

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

	#[tracing::instrument(skip(self))]
	pub fn sub_cell_at(&self, options: FactoredOffsetCellOptions<u8>) -> Result<(), AssemblyError> {
		let subtractor = {
			let current_cell = self.take(0)?;

			self.resolve_factor(current_cell, options.factor())?
		};

		let (other_value, gep) = self.load_cell_and_pointer(options.offset())?;

		let value_to_store =
			self.builder
				.build_int_sub(other_value, subtractor, "sub_cell_at_sub\0")?;

		self.store_into(value_to_store, gep)?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub fn sub_from_cell(
		&self,
		options: FactoredOffsetCellOptions<u8>,
	) -> Result<(), AssemblyError> {
		let subtractor = {
			let current_cell = self.take(options.offset())?;

			self.resolve_factor(current_cell, options.factor())?
		};

		let (other_value, gep) = self.load_cell_and_pointer(0)?;

		let value_to_store =
			self.builder
				.build_int_sub(other_value, subtractor, "sub_from_cell_sub\0")?;

		self.store_into(value_to_store, gep)?;

		Ok(())
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

		let vec_of_indices = {
			let offsets = values.iter().map(|x| x.offset()).collect::<Vec<_>>();

			self.offset_many_pointers(&offsets)?
		};

		let vec_of_pointers = unsafe {
			self.builder.build_vec_gep(
				i8_type,
				self.pointers.tape,
				vec_of_indices,
				"duplicate_cell_gep\0",
			)?
		};

		let vec_of_loaded_values = self.call_vector_gather(i8_vec_type, vec_of_pointers)?;

		let vec_of_modified_values = if values.iter().all(|x| matches!(x.factor(), 1)) {
			self.builder.build_int_add(
				vec_of_current_cell,
				vec_of_loaded_values,
				"duplicate_cell_vector_add\0",
			)?
		} else {
			// let vec_of_factors = {
			// 	let factors = values
			// 		.iter()
			// 		.map(|x| i8_type.const_int(x.factor() as u64, false))
			// 		.collect::<Vec<_>>();

			// 	VectorType::const_vector(&factors)
			// };

			// let vec_of_scaled_current_cell = self.builder.build_int_mul(
			// 	vec_of_current_cell,
			// 	vec_of_factors,
			// 	"duplicate_cell_vector_mul\0",
			// )?;

			let vec_of_scaled_current_cell = if values.iter().all(|x| {
				let factor = x.factor();

				factor.is_positive() && (factor as u64).is_power_of_two()
			}) {
				let vec_of_factors = {
					let factors = values
						.iter()
						.map(|x| {
							i8_type.const_int((x.factor() as u8).ilog2().convert::<u64>(), false)
						})
						.collect::<Vec<_>>();

					VectorType::const_vector(&factors)
				};

				self.builder.build_left_shift(
					vec_of_current_cell,
					vec_of_factors,
					"duplicate_cell_vector_shl\0",
				)?
			} else {
				let vec_of_factors = {
					let factors = values
						.iter()
						.map(|x| i8_type.const_int(x.factor() as u64, false))
						.collect::<Vec<_>>();

					VectorType::const_vector(&factors)
				};

				self.builder.build_int_mul(
					vec_of_current_cell,
					vec_of_factors,
					"duplicate_cell_vector_mul\0",
				)?
			};

			self.builder.build_int_add(
				vec_of_loaded_values,
				vec_of_scaled_current_cell,
				"duplicate_cell_vector_add\0",
			)?
		};

		self.call_vector_scatter(vec_of_modified_values, vec_of_pointers)
	}

	#[tracing::instrument(skip(self))]
	pub fn set_many_cells(&self, options: &SetManyCellsOptions) -> Result<(), AssemblyError> {
		let i8_type = self.context().i8_type();

		let vec_of_indices = {
			let offsets = options
				.iter()
				.map(ValuedOffsetCellOptions::offset)
				.collect::<Vec<_>>();

			self.offset_many_pointers(&offsets)?
		};

		let vec_of_pointers = unsafe {
			self.builder.build_vec_gep(
				i8_type,
				self.pointers.tape,
				vec_of_indices,
				"set_many_cells_gep\0",
			)?
		};

		let vec_to_store = {
			let values = options
				.values()
				.iter()
				.map(|&x| i8_type.const_int(x.convert::<u64>(), false))
				.collect::<Vec<_>>();

			VectorType::const_vector(&values)
		};

		self.call_vector_scatter(vec_to_store, vec_of_pointers)
	}

	#[tracing::instrument(skip(self))]
	pub fn set_range(&self, options: SetRangeOptions) -> Result<(), AssemblyError> {
		let i8_type = self.context().i8_type();

		let vec_of_indices = {
			let offsets = options
				.iter()
				.map(ValuedOffsetCellOptions::offset)
				.collect::<Vec<_>>();

			self.offset_many_pointers(&offsets)?
		};

		let vec_of_pointers = unsafe {
			self.builder.build_vec_gep(
				i8_type,
				self.pointers.tape,
				vec_of_indices,
				"set_range_gep\0",
			)?
		};

		let vec_to_store = {
			let values = options
				.range()
				.map(|_| i8_type.const_int(options.value().convert::<u64>(), false))
				.collect::<Vec<_>>();

			VectorType::const_vector(&values)
		};

		self.call_vector_scatter(vec_to_store, vec_of_pointers)
	}

	#[tracing::instrument(skip(self))]
	pub fn change_many_cells(&self, options: &ChangeManyCellsOptions) -> Result<(), AssemblyError> {
		let i8_type = self.context().i8_type();
		let i8_vec_type = i8_type.vec_type(options.values().len() as u32);

		let vec_of_indices = {
			let offsets = options
				.iter()
				.map(ValuedOffsetCellOptions::offset)
				.collect::<Vec<_>>();

			self.offset_many_pointers(&offsets)?
		};

		let vec_of_pointers = unsafe {
			self.builder.build_vec_gep(
				i8_type,
				self.pointers.tape,
				vec_of_indices,
				"change_many_cells_gep\0",
			)?
		};

		let vec_of_loaded_values = self.call_vector_gather(i8_vec_type, vec_of_pointers)?;

		let vec_of_offsets = {
			let values = options
				.values()
				.iter()
				.map(|&x| i8_type.const_int(x as u64, false))
				.collect::<Vec<_>>();

			VectorType::const_vector(&values)
		};

		let vec_to_store = self.builder.build_int_add(
			vec_of_loaded_values,
			vec_of_offsets,
			"change_many_cells_add\0",
		)?;

		self.call_vector_scatter(vec_to_store, vec_of_pointers)
	}
}
