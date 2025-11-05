mod cells;
mod chars;

use frick_ir::{BrainIr, OutputOptions, ValuedOffsetCellOptions};
use frick_utils::Convert as _;
use inkwell::{
	context::ContextRef,
	values::{BasicMetadataValueEnum, BasicValueEnum, InstructionValueError, IntValue},
};

use crate::{AssemblyError, ContextGetter as _, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip(self))]
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

		let range_metadata_node = self.context().metadata_node(&[
			i32_i8_min.convert::<BasicMetadataValueEnum<'ctx>>(),
			i32_i8_max.convert::<BasicMetadataValueEnum<'ctx>>(),
		]);

		char_instr.set_metadata(range_metadata_node, range_metadata_id)
	}

	pub fn input_into_cell(
		&self,
		input_options: ValuedOffsetCellOptions<i8>,
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();

		let getchar_call =
			self.builder
				.build_call(self.functions.getchar, &[], "input_into_cell_call\0")?;

		getchar_call.set_tail_call(true);

		let getchar_value = getchar_call
			.try_as_basic_value()
			.unwrap_basic()
			.into_int_value();

		self.add_range_io_metadata(
			context,
			getchar_value,
			u8::MIN.convert::<u64>(),
			u8::MAX.convert::<u64>(),
		)?;

		let truncated_value = self.builder.build_int_truncate(
			getchar_value,
			i8_type,
			"input_into_cell_truncate\0",
		)?;

		let offset_truncated_value = if matches!(input_options.value(), 0) {
			truncated_value
		} else {
			let offset_value = i8_type.const_int(input_options.value() as u64, false);

			self.builder.build_int_nsw_add(
				truncated_value,
				offset_value,
				"input_into_cell_add\0",
			)?
		};

		self.store_into_cell(offset_truncated_value, input_options.offset())?;

		Ok(())
	}

	fn call_putchar(
		&self,
		context: ContextRef<'ctx>,
		value: IntValue<'ctx>,
	) -> Result<(), AssemblyError> {
		let continue_block =
			context.append_basic_block(self.functions.main, "putchar.invoke.cont\0");

		let call = self.builder.build_direct_invoke(
			self.functions.putchar,
			&[value.convert::<BasicValueEnum<'ctx>>()],
			continue_block,
			self.catch_block,
			"putchar_invoke\0",
		)?;

		call.set_tail_call(true);

		self.builder.position_at_end(continue_block);

		Ok(())
	}
}
