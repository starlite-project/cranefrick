mod cells;
mod chars;

use frick_ir::{BrainIr, OutputOptions};
use inkwell::{
	context::ContextRef,
	values::{InstructionValueError, IntValue, PointerValue},
};

use crate::{AssemblyError, ContextGetter as _, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn output(&self, options: &OutputOptions) -> Result<(), AssemblyError> {
		match options {
			OutputOptions::Cell(options) => self.output_cell(*options)?,
			OutputOptions::Cells(options) => self.output_cells(options)?,
			OutputOptions::Char(c) => self.output_char(*c)?,
			OutputOptions::Str(c) => self.output_str(c)?,
			_ => {
				return Err(AssemblyError::NotImplemented(BrainIr::Output(
					options.clone(),
				)));
			}
		}

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

	pub fn input_into_cell(&self) -> Result<(), AssemblyError> {
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
	) -> Result<IntValue<'ctx>, AssemblyError> {
		let continue_block =
			context.append_basic_block(self.functions.main, &format!("{fn_name}.puts.invoke.cont"));

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

	fn call_putchar(
		&self,
		context: ContextRef<'ctx>,
		value: IntValue<'ctx>,
		fn_name: &'static str,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		let continue_block = context.append_basic_block(
			self.functions.main,
			&format!("{fn_name}.putchar.invoke.cont"),
		);

		let call = self.builder.build_direct_invoke(
			self.functions.putchar,
			&[value.into()],
			continue_block,
			self.catch_block,
			&format!("{fn_name}_putchar_invoke"),
		)?;

		call.set_tail_call(true);

		self.builder.position_at_end(continue_block);

		Ok(call.try_as_basic_value().unwrap_left().into_int_value())
	}
}
