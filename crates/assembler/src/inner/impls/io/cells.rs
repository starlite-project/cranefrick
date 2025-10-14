use frick_ir::{ChangeCellOptions, Value};
use inkwell::types::IntType;

use crate::{
	AssemblyError, ContextGetter as _,
	inner::{InnerAssembler, utils::OUTPUT_ARRAY_LEN},
};

impl<'ctx> InnerAssembler<'ctx> {
	pub(super) fn output_cell(
		&self,
		options: ChangeCellOptions<i8, Value>,
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

	pub(super) fn output_cells(
		&self,
		options: &[ChangeCellOptions<i8, Value>],
	) -> Result<(), AssemblyError> {
		if options.len() <= OUTPUT_ARRAY_LEN as usize {
			self.output_cells_puts(options)
		} else {
			self.output_cells_iterated(options)
		}
	}

	fn output_cells_iterated(
		&self,
		options: &[ChangeCellOptions<i8, Value>],
	) -> Result<(), AssemblyError> {
		options
			.chunks(OUTPUT_ARRAY_LEN as usize)
			.try_for_each(|x| self.output_cells(x))
	}

	fn output_cells_puts(
		&self,
		options: &[ChangeCellOptions<i8, Value>],
	) -> Result<(), AssemblyError> {
		assert!(options.len() <= OUTPUT_ARRAY_LEN as usize);

		let context = self.context();

		let i8_type = context.i8_type();
		let i64_type = context.i64_type();

		let _lifetime = {
			let lifetime_array_len = i64_type.const_int(options.len() as u64, false);

			self.start_lifetime(lifetime_array_len, self.pointers.output)?
		};

		if is_memcpyable(options) {
			self.setup_output_cells_puts_memcpy(i8_type, i64_type, options)
		} else if is_memsettable(options) {
			self.setup_output_cells_puts_memset(i8_type, i64_type, options[0], options.len() as u64)
		} else {
			self.setup_output_cells_puts_iterated(i8_type, options)
		}?;

		self.call_puts(
			context,
			self.pointers.output,
			options.len() as u64,
			"output_cells_puts",
		)?;

		Ok(())
	}

	fn setup_output_cells_puts_memcpy(
		&self,
		i8_type: IntType<'ctx>,
		i64_type: IntType<'ctx>,
		options: &[ChangeCellOptions<i8, Value>],
	) -> Result<(), AssemblyError> {
		let start = options.first().unwrap().offset();
		let len = (start..=options.last().unwrap().offset()).count() as u32;

		let current_gep = self.tape_gep(i8_type, start, "setup_output_cells_puts_memcpy")?;

		let len_value = i64_type.const_int(len.into(), false);

		self.builder
			.build_memcpy(self.pointers.output, 16, current_gep, 16, len_value)?;

		Ok(())
	}

	fn setup_output_cells_puts_memset(
		&self,
		i8_type: IntType<'ctx>,
		i64_type: IntType<'ctx>,
		options: ChangeCellOptions<i8, Value>,
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

	fn setup_output_cells_puts_iterated(
		&self,
		i8_type: IntType<'ctx>,
		options: &[ChangeCellOptions<i8, Value>],
	) -> Result<(), AssemblyError> {
		let ptr_int_type = self.ptr_int_type;

		for (i, char) in options.iter().copied().enumerate() {
			let loaded_char = self.load(char.offset(), "setup_output_cells_puts_iterated")?;

			let offset_char = if matches!(char.value(), 0) {
				loaded_char
			} else {
				let offset_value = i8_type.const_int(char.value() as u64, false);

				self.builder.build_int_add(
					loaded_char,
					offset_value,
					"setup_output_cells_puts_iterated_add\0",
				)?
			};

			let array_offset = ptr_int_type.const_int(i as u64, false);

			let output_array_gep = self.gep(
				i8_type,
				self.pointers.output,
				array_offset,
				"setup_output_cells_puts_iterated",
			)?;

			self.store_into(offset_char, output_array_gep)?;
		}

		Ok(())
	}
}

fn is_memcpyable(options: &[ChangeCellOptions<i8, Value>]) -> bool {
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

fn is_memsettable(options: &[ChangeCellOptions<i8, Value>]) -> bool {
	if options.len() <= 1 {
		return false;
	}

	options.windows(2).all(|w| w[0] == w[1])
}
