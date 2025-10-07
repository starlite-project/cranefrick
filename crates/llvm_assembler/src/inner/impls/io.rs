use std::slice;

use frick_assembler::AssemblyError;
use frick_ir::{BrainIr, CellChangeOptions, OutputOptions};
use inkwell::{
	context::ContextRef,
	types::IntType,
	values::{InstructionValueError, IntValue, PointerValue},
};

use crate::{
	LlvmAssemblyError,
	inner::{InnerAssembler, utils::OUTPUT_ARRAY_LEN},
};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn output(&self, options: &OutputOptions) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		match options {
			OutputOptions::Cell(options) => {
				self.output_cells_puts(slice::from_ref(options))?;
			}
			OutputOptions::Cells(options) => self.output_cells(options)?,
			OutputOptions::Char(c) => self.output_chars(slice::from_ref(c))?,
			OutputOptions::Str(c) => self.output_chars(c)?,
			_ => {
				return Err(AssemblyError::NotImplemented(BrainIr::Output(
					options.clone(),
				)));
			}
		}

		Ok(())
	}

	fn output_cells(&self, options: &[CellChangeOptions<i8>]) -> Result<(), LlvmAssemblyError> {
		if options.len() <= OUTPUT_ARRAY_LEN as usize {
			self.output_cells_puts(options)
		} else {
			self.output_cells_iterated(options)
		}
	}

	fn output_cells_puts(
		&self,
		options: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
		assert!(options.len() <= OUTPUT_ARRAY_LEN as usize);

		let context = self.context();

		let i8_type = context.i8_type();
		let i64_type = context.i64_type();

		let _lifetime = {
			let lifetime_array_len = i64_type.const_int(options.len() as u64, false);

			self.start_lifetime(lifetime_array_len, self.pointers.output)?
		};

		if is_memcpyable(options) {
			self.setup_output_cells_puts_memcpy(context, options)
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
		context: ContextRef<'ctx>,
		options: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
		let i8_type = context.i8_type();

		let start = options.first().unwrap().offset();
		let len = (start..=options.last().unwrap().offset()).count() as u32;

		let current_gep = self.tape_gep(i8_type, start, "setup_output_cells_puts_memcpy")?;

		let len_value = {
			let i64_type = context.i64_type();

			i64_type.const_int(len.into(), false)
		};

		self.builder
			.build_memcpy(self.pointers.output, 1, current_gep, 1, len_value)?;

		Ok(())
	}

	fn setup_output_cells_puts_memset(
		&self,
		i8_type: IntType<'ctx>,
		i64_type: IntType<'ctx>,
		options: CellChangeOptions<i8>,
		length: u64,
	) -> Result<(), LlvmAssemblyError> {
		let current_value = self.load(options.offset(), "setup_output_cells_puts_memset")?;

		let value_to_memset = if matches!(options.value(), 0) {
			current_value
		} else {
			let offset = i8_type.const_int(options.value() as u64, false);

			self.builder.build_int_add(
				current_value,
				offset,
				"setup_output_cells_puts_memset_add",
			)?
		};

		let array_len = i64_type.const_int(length, false);

		self.builder
			.build_memset(self.pointers.output, 1, value_to_memset, array_len)?;

		Ok(())
	}

	fn setup_output_cells_puts_iterated(
		&self,
		i8_type: IntType<'ctx>,
		options: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
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
					"setup_output_cells_puts_iterated_add",
				)?
			};

			let array_offset = ptr_int_type.const_int(i as u64, false);

			let output_array_gep = unsafe {
				self.builder.build_in_bounds_gep(
					i8_type,
					self.pointers.output,
					&[array_offset],
					"setup_output_cells_puts_iterated_gep",
				)?
			};

			self.store_into(offset_char, output_array_gep)?;
		}

		Ok(())
	}

	fn output_cells_iterated(
		&self,
		options: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
		options
			.chunks(OUTPUT_ARRAY_LEN as usize)
			.try_for_each(|x| self.output_cells(x))
	}

	fn output_chars(&self, c: &[u8]) -> Result<(), LlvmAssemblyError> {
		let context = self.context();

		let constant_initializer = context.const_string(c, false);

		let constant_string_ty = constant_initializer.get_type();

		let global_constant =
			self.module
				.add_global(constant_string_ty, None, "output_chars_global_value");

		self.setup_global_value(global_constant, &constant_initializer);

		let global_constant_pointer = global_constant.as_pointer_value();

		let puts_value = self.call_puts(
			context,
			global_constant_pointer,
			c.len() as u64,
			"output_chars",
		)?;

		let last = c.last().copied().unwrap();

		self.add_range_io_metadata(context, puts_value, last.into(), last.into())?;

		Ok(())
	}

	fn add_range_io_metadata(
		&self,
		context: ContextRef<'ctx>,
		char_call: IntValue<'ctx>,
		min_inclusive: u64,
		max_inclusive: u64,
	) -> Result<(), InstructionValueError> {
		let Some(char_instr) = char_call.as_instruction() else {
			return Ok(());
		};

		let i32_type = context.i32_type();

		let i32_i8_min = i32_type.const_int(min_inclusive, false);

		let i32_i8_max = i32_type.const_int(max_inclusive + 1, false);

		let range_metadata_id = context.get_kind_id("range");

		let range_metadata_node = self
			.context()
			.metadata_node(&[i32_i8_min.into(), i32_i8_max.into()]);

		char_instr.set_metadata(range_metadata_node, range_metadata_id)
	}

	pub fn input_into_cell(&self) -> Result<(), LlvmAssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();

		let getchar_call =
			self.builder
				.build_call(self.functions.getchar, &[], "input_into_cell_call")?;

		getchar_call.set_tail_call(true);

		let getchar_value = getchar_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		self.add_range_io_metadata(context, getchar_value, u8::MIN.into(), u8::MAX.into())?;

		let truncated_value =
			self.builder
				.build_int_truncate(getchar_value, i8_type, "input_into_cell_truncate")?;

		self.store(truncated_value, 0, "input_into_cell")
	}

	fn call_puts(
		&self,
		context: ContextRef<'ctx>,
		array_ptr: PointerValue<'ctx>,
		array_len: u64,
		fn_name: &'static str,
	) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let continue_block =
			context.append_basic_block(self.functions.main, &format!("{fn_name}.invoke.cont"));

		let array_len_value = {
			let i64_type = context.i64_type();

			i64_type.const_int(array_len, false)
		};

		let call = self.builder.build_direct_invoke(
			self.functions.puts,
			&[array_ptr.into(), array_len_value.into()],
			continue_block,
			self.catch_block,
			&format!("{fn_name}_puts_invoke"),
		)?;

		call.set_tail_call(true);

		self.builder.position_at_end(continue_block);

		Ok(call.try_as_basic_value().unwrap_left().into_int_value())
	}
}

fn is_memcpyable(options: &[CellChangeOptions<i8>]) -> bool {
	if options.len() <= 1 {
		return false;
	}

	if options.iter().any(|x| !matches!(x.value(), 0)) {
		return false;
	}

	options
		.windows(2)
		.all(|w| w[0].offset() + 1 == w[1].offset())
}

fn is_memsettable(options: &[CellChangeOptions<i8>]) -> bool {
	if options.len() <= 1 {
		return false;
	}

	options.windows(2).all(|w| w[0] == w[1])
}
