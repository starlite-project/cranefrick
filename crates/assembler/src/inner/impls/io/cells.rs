use frick_ir::ValuedOffsetCellOptions;
use frick_utils::SliceExt as _;
use inkwell::{types::VectorType, values::VectorValue};

use crate::{AssemblyError, BuilderExt as _, ContextGetter as _, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip_all)]
	pub(super) fn output_cell(
		&self,
		options: ValuedOffsetCellOptions<i8>,
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let _invariant = self.start_tape_invariant()?;

		let offset_cell_value = self.load_valued_offset_cell(options)?;

		self.call_putchar(context, offset_cell_value)
	}

	#[tracing::instrument(skip(self))]
	pub(super) fn output_cells(
		&self,
		options: &[ValuedOffsetCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		if options.windows_n::<2>().all(|&[x, y]| x == y) {
			self.output_cell_many_times(options[0], options.len())
		} else {
			self.output_many_cells(options)
		}
	}

	pub(super) fn output_cell_many_times(
		&self,
		options: ValuedOffsetCellOptions<i8>,
		count: usize,
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let range = 0..count;

		let cell_value = self.load_valued_offset_cell(options)?;

		for _ in range {
			self.call_putchar(context, cell_value)?;
		}

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub(super) fn output_many_cells(
		&self,
		options: &[ValuedOffsetCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let i64_type = context.i64_type();

		let _invariant = self.start_tape_invariant()?;

		let output_vector = self.get_output_cells_vector(options)?;

		let mut char_values = Vec::with_capacity(options.len());

		for i in options.iter().enumerate().map(|(i, ..)| i) {
			let index = i64_type.const_int(i as u64, false);

			let current_char = self
				.builder
				.build_extract_element(output_vector, index, "output_cells_vector_index\0")?
				.into_int_value();

			char_values.push(current_char);
		}

		char_values
			.into_iter()
			.try_for_each(|ch| self.call_putchar(context, ch))
	}

	#[tracing::instrument(skip_all)]
	fn get_output_cells_vector(
		&self,
		options: &[ValuedOffsetCellOptions<i8>],
	) -> Result<VectorValue<'ctx>, AssemblyError> {
		if is_splattable(options) {
			self.get_output_cells_vector_splat(options)
		} else {
			self.get_output_cells_vector_scatter(options)
		}
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

			self.builder.build_int_add(
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

			self.builder.build_int_add(
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

	options
		.windows_n::<2>()
		.all(|&[x, y]| x.offset() == y.offset())
}
