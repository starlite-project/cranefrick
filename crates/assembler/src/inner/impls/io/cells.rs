use frick_ir::ValuedOffsetCellOptions;
use inkwell::{types::VectorType, values::VectorValue};

use crate::{
	AssemblyError, BuilderExt as _, ContextGetter as _,
	inner::{InnerAssembler, utils::is_contiguous},
};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip_all)]
	pub(super) fn output_cell(
		&self,
		options: ValuedOffsetCellOptions<i8>,
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i32_type = context.i32_type();

		let _invariant = self.start_tape_invariant()?;

		let current_cell_value = self.load_cell(options.offset())?;

		let offset_cell_value = if matches!(options.value(), 0) {
			current_cell_value
		} else {
			let offset_value = i8_type.const_int(options.value() as u64, false);

			self.builder
				.build_int_nsw_add(current_cell_value, offset_value, "output_cell_add\0")?
		};

		let extended_value = self.builder.build_int_z_extend_or_bit_cast(
			offset_cell_value,
			i32_type,
			"output_cell_extend\0",
		)?;

		if let Some(extend_instr) = extended_value.as_instruction() {
			extend_instr.set_non_negative_flag(true);
		}

		self.call_putchar(context, extended_value)
	}

	#[tracing::instrument(skip_all)]
	pub(super) fn output_cells(
		&self,
		options: &[ValuedOffsetCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let i32_type = context.i32_type();
		let i64_type = context.i64_type();

		let _invariant = self.start_tape_invariant()?;

		let output_vector = self.get_output_cells_vector(options)?;

		for i in options.iter().enumerate().map(|(i, ..)| i) {
			let index = i64_type.const_int(i as u64, false);

			let current_char = self
				.builder
				.build_extract_element(output_vector, index, "output_cells_vector_index\0")?
				.into_int_value();

			let extended_char = self.builder.build_int_z_extend_or_bit_cast(
				current_char,
				i32_type,
				"output_cells_extend_char\0",
			)?;

			if let Some(extend_instr) = extended_char.as_instruction() {
				extend_instr.set_non_negative_flag(true);
			}

			self.call_putchar(context, extended_char)?;
		}

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	fn get_output_cells_vector(
		&self,
		options: &[ValuedOffsetCellOptions<i8>],
	) -> Result<VectorValue<'ctx>, AssemblyError> {
		if is_splattable(options) {
			self.get_output_cells_vector_splat(options)
		} else if is_contiguous(options) {
			self.get_output_cells_vector_contiguous(options)
		} else {
			self.get_output_cells_vector_scatter(options)
		}
	}

	#[tracing::instrument(skip_all)]
	fn get_output_cells_vector_contiguous(
		&self,
		options: &[ValuedOffsetCellOptions<i8>],
	) -> Result<VectorValue<'ctx>, AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i8_vec_type = i8_type.vec_type(options.len() as u32);

		let initial_cell_offset = options[0].offset();

		let initial_cell_gep = self.tape_gep(i8_vec_type, initial_cell_offset)?;

		let vec_of_loaded_cells = self.load_from(i8_vec_type, initial_cell_gep)?;

		Ok(if options.iter().all(|x| matches!(x.value(), 0)) {
			vec_of_loaded_cells
		} else {
			let vec_of_value_offsets = {
				let vec_of_value_offsets = options
					.iter()
					.map(|v| i8_type.const_int(v.value() as u64, false))
					.collect::<Vec<_>>();

				VectorType::const_vector(&vec_of_value_offsets)
			};

			self.builder.build_int_nsw_add(
				vec_of_loaded_cells,
				vec_of_value_offsets,
				"get_output_cells_vector_contiguous_offset_values\0",
			)?
		})
	}

	#[tracing::instrument(skip_all)]
	fn get_output_cells_vector_splat(
		&self,
		options: &[ValuedOffsetCellOptions<i8>],
	) -> Result<VectorValue<'ctx>, AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let i8_vec_type = i8_type.vec_type(options.len() as u32);
		let i32_vec_type = i32_type.vec_type(options.len() as u32);

		let cell_to_load_offset = options[0].offset();

		let loaded_cell_value = self.load_cell(cell_to_load_offset)?;

		let vec_of_loaded_cell = {
			let i64_zero = i64_type.const_zero();

			let tmp = self.builder.build_insert_element(
				i8_vec_type.get_poison(),
				loaded_cell_value,
				i64_zero,
				"get_output_cells_vector_splat_insert_initial_element\0",
			)?;

			self.builder.build_shuffle_vector(
				tmp,
				i8_vec_type.get_poison(),
				i32_vec_type.const_zero(),
				"get_output_cells_vector_splat_vector\0",
			)?
		};

		Ok(if options.iter().all(|x| matches!(x.value(), 0)) {
			vec_of_loaded_cell
		} else {
			let vec_of_value_offsets = {
				let vec_of_value_offsets = options
					.iter()
					.map(|v| i8_type.const_int(v.value() as u64, false))
					.collect::<Vec<_>>();

				VectorType::const_vector(&vec_of_value_offsets)
			};

			self.builder.build_int_nsw_add(
				vec_of_loaded_cell,
				vec_of_value_offsets,
				"get_output_cells_vector_splat_offset_values\0",
			)?
		})
	}

	#[tracing::instrument(skip_all)]
	fn get_output_cells_vector_scatter(
		&self,
		options: &[ValuedOffsetCellOptions<i8>],
	) -> Result<VectorValue<'ctx>, AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i8_vec_type = i8_type.vec_type(options.len() as u32);

		let vec_of_indices = {
			let offsets = options.iter().map(|x| x.offset()).collect::<Vec<_>>();

			self.offset_many_pointers(&offsets)?
		};

		let vec_of_pointers = unsafe {
			self.builder.build_vec_gep(
				i8_type,
				self.pointers.tape,
				vec_of_indices,
				"get_output_cells_vector_scatter_gep\0",
			)?
		};

		let vec_of_loaded_values = self.call_vector_gather(i8_vec_type, vec_of_pointers)?;

		Ok(if options.iter().all(|x| matches!(x.value(), 0)) {
			vec_of_loaded_values
		} else {
			let vec_of_value_offsets = {
				let vec_of_value_offsets = options
					.iter()
					.map(|v| i8_type.const_int(v.value() as u64, false))
					.collect::<Vec<_>>();

				VectorType::const_vector(&vec_of_value_offsets)
			};

			self.builder.build_int_nsw_add(
				vec_of_loaded_values,
				vec_of_value_offsets,
				"get_output_cells_vector_scatter_offset\0",
			)?
		})
	}
}

fn is_splattable(options: &[ValuedOffsetCellOptions<i8>]) -> bool {
	if options.len() <= 1 {
		return false;
	}

	options.windows(2).all(|w| w[0].offset() == w[1].offset())
}
