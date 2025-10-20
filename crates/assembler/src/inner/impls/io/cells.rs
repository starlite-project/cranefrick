use frick_ir::ValuedChangeCellOptions;
use inkwell::types::{IntType, VectorType};

use crate::{
	AssemblyError, BuilderExt as _, ContextGetter as _,
	inner::{InnerAssembler, utils::OUTPUT_ARRAY_LEN},
};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip_all)]
	pub(super) fn output_cell(
		&self,
		options: ValuedChangeCellOptions<i8>,
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i32_type = context.i32_type();

		let current_cell_value = self.load(options.offset(), "output_cell")?;

		let offset_cell_value = if matches!(options.value(), 0) {
			current_cell_value
		} else {
			let offset_value = i8_type.const_int(options.value() as u64, false);

			self.builder
				.build_int_add(current_cell_value, offset_value, "output_cell_add\0")?
		};

		let extended_value = self.builder.build_int_z_extend_or_bit_cast(
			offset_cell_value,
			i32_type,
			"output_cell_extend\0",
		)?;

		self.call_putchar(context, extended_value, "output_cell")?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub(super) fn output_cells(
		&self,
		options: &[ValuedChangeCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		if options.len() <= OUTPUT_ARRAY_LEN as usize {
			tracing::debug!("output cells with frick_puts");
			self.output_cells_puts(options)
		} else {
			tracing::debug!("unable to output cells with frick_puts");
			self.output_cells_iterated(options)
		}
	}

	#[tracing::instrument(skip_all)]
	fn output_cells_iterated(
		&self,
		options: &[ValuedChangeCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		options
			.chunks(OUTPUT_ARRAY_LEN as usize)
			.try_for_each(|x| self.output_cells(x))
	}

	#[tracing::instrument(skip_all)]
	fn output_cells_puts(
		&self,
		options: &[ValuedChangeCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		assert!(options.len() <= OUTPUT_ARRAY_LEN as usize);

		let context = self.context();

		let i8_type = context.i8_type();
		let i64_type = context.i64_type();

		let _output_lifetime = self.start_output_lifetime(options.len() as u64)?;
		let tape_invariant = self.start_tape_invariant()?;

		if is_memcpyable(options) {
			tracing::debug!("memcpying cells into output array");
			self.setup_output_cells_puts_memcpy(i8_type, i64_type, options)
		} else if is_memsettable(options) {
			tracing::debug!("unable to memcpy cells");
			tracing::debug!("memsetting cells of output array");
			self.setup_output_cells_puts_memset(i8_type, i64_type, options[0], options.len() as u64)
		} else {
			tracing::debug!("unable to memcpy or memset cells");
			self.setup_output_cells_puts_vector_scattered(options)
		}?;

		let _output_invariant = {
			let array_len_value = i64_type.const_int(options.len() as u64, false);

			self.start_invariant(array_len_value, self.pointers.output)?
		};

		self.call_puts(
			context,
			self.pointers.output,
			options.len() as u64,
			"output_cells_puts",
		)?;

		drop(tape_invariant);

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	fn setup_output_cells_puts_memcpy(
		&self,
		i8_type: IntType<'ctx>,
		i64_type: IntType<'ctx>,
		options: &[ValuedChangeCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		let start = options.first().unwrap().offset();
		let len = (start..=options.last().unwrap().offset()).count() as u32;

		let current_gep = self.tape_gep(i8_type, start, "setup_output_cells_puts_memcpy")?;

		let len_value = i64_type.const_int(len.into(), false);

		self.builder
			.build_memcpy(self.pointers.output, 1, current_gep, 1, len_value)?;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	fn setup_output_cells_puts_memset(
		&self,
		i8_type: IntType<'ctx>,
		i64_type: IntType<'ctx>,
		options: ValuedChangeCellOptions<i8>,
		length: u64,
	) -> Result<(), AssemblyError> {
		let current_value = self.load(options.offset(), "setup_output_cells_puts_memset")?;

		let value_to_memset = if matches!(options.value(), 0) {
			current_value
		} else {
			let value_offset = i8_type.const_int(options.value() as u64, false);

			self.builder.build_int_add(
				current_value,
				value_offset,
				"setup_output_cells_puts_memset_add\0",
			)?
		};

		let array_len = i64_type.const_int(length, false);

		self.builder
			.build_memset(self.pointers.output, 16, value_to_memset, array_len)?;

		Ok(())
	}

	#[tracing::instrument(skip_all, fields(options = options.len()))]
	fn setup_output_cells_puts_vector_scattered(
		&self,
		options: &[ValuedChangeCellOptions<i8>],
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let ptr_int_type = self.ptr_int_type;
		let bool_type = context.bool_type();
		let i8_type = context.i8_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let i8_vec_type = i8_type.vec_type(options.len() as u32);
		let i32_vec_type = i32_type.vec_type(options.len() as u32);
		let ptr_int_vec_type = ptr_int_type.vec_type(options.len() as u32);

		let vec_of_indices = {
			if options.windows(2).all(|x| x[0].offset() == x[1].offset()) {
				let offset = self.offset_pointer(options[0].offset())?;

				let zero_index = i64_type.const_zero();

				let tmp = self.builder.build_insert_element(
					ptr_int_vec_type.get_undef(),
					offset,
					zero_index,
					"setup_output_cells_puts_vector_scattered_insert_element\0",
				)?;

				self.builder.build_shuffle_vector(
					tmp,
					ptr_int_vec_type.get_undef(),
					i32_vec_type.const_zero(),
					"setup_output_cells_puts_vector_scattered_shuffle_vector\0",
				)?
			} else {
				let mut vec = ptr_int_vec_type.get_undef();

				for (i, offset) in options
					.iter()
					.copied()
					.map(ValuedChangeCellOptions::offset)
					.enumerate()
				{
					let index = i64_type.const_int(i as u64, false);

					let offset = self.offset_pointer(offset)?;

					vec = self.builder.build_insert_element(
						vec,
						offset,
						index,
						"setup_output_cells_puts_vector_scattered_insert_element\0",
					)?;
				}

				vec
			}
		};

		let vec_of_ptrs = unsafe {
			self.builder.build_vec_gep(
				i8_type,
				self.pointers.tape,
				vec_of_indices,
				"setup_output_cells_puts_vector_scattered_gep\0",
			)?
		};

		let vector_gather = self.get_vector_gather(i8_vec_type)?;

		let vec_load_alignment = i32_type.const_int(1, false);

		let bool_vec_all_on = {
			let vec_of_trues = vec![bool_type.const_all_ones(); options.len()];

			VectorType::const_vector(&vec_of_trues)
		};

		let vec_of_loaded_values = self
			.builder
			.build_call(
				vector_gather,
				&[
					vec_of_ptrs.into(),
					vec_load_alignment.into(),
					bool_vec_all_on.into(),
					i8_vec_type.get_undef().into(),
				],
				"setup_output_cells_puts_vector_scattered_vector_load_call\0",
			)?
			.try_as_basic_value()
			.unwrap_left()
			.into_vector_value();

		let vec_of_values_for_output = if options
			.iter()
			.copied()
			.map(ValuedChangeCellOptions::value)
			.all(|x| matches!(x, 0))
		{
			vec_of_loaded_values
		} else {
			let vec_of_value_offsets = {
				let mut vec = i8_vec_type.const_zero();

				for (i, value_offset) in options
					.iter()
					.copied()
					.map(ValuedChangeCellOptions::value)
					.enumerate()
				{
					let index = i64_type.const_int(i as u64, false);

					let value_offset = i8_type.const_int(value_offset as u64, false);

					vec = vec
						.const_insert_element(index, value_offset)
						.into_vector_value();
				}

				vec
			};

			self.builder.build_int_add(
				vec_of_loaded_values,
				vec_of_value_offsets,
				"setup_output_cells_puts_vector_scattered_add\0",
			)?
		};

		self.store_into(vec_of_values_for_output, self.pointers.output)
	}
}

fn is_memcpyable(options: &[ValuedChangeCellOptions<i8>]) -> bool {
	if options.len() <= 1 {
		return false;
	}

	if options.iter().any(|x| matches!(x.value(), 0)) {
		return false;
	}

	options
		.windows(2)
		.all(|w| w[0].offset() + 1 == w[1].offset())
}

fn is_memsettable(options: &[ValuedChangeCellOptions<i8>]) -> bool {
	if options.len() <= 1 {
		return false;
	}

	options.windows(2).all(|w| w[0] == w[1])
}
