use frick_assembler::AssemblyError;
use frick_ir::{BrainIr, CellChangeOptions, OutputOptions};
use inkwell::{
	attributes::{Attribute, AttributeLoc},
	values::{InstructionValueError, IntValue},
};

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn output(&self, options: &OutputOptions) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		match options {
			OutputOptions::Cell(options) => {
				self.output_current_cell(options.value(), options.offset())?;
			}
			OutputOptions::Cells(options) => self.output_cells(options)?,
			OutputOptions::Char(c) => self.output_char(*c)?,
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
		options
			.iter()
			.copied()
			.try_for_each(|x| self.output_current_cell(x.value(), x.offset()))
	}

	fn output_current_cell(&self, value_offset: i8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i32_type = self.context().i32_type();
		let loaded_value = self.load(offset, "output_current_cell")?;

		let extended_loaded_value = self.builder.build_int_z_extend(
			loaded_value,
			i32_type,
			"output_current_cell_extend",
		)?;

		if let Some(extended_instr) = extended_loaded_value.as_instruction() {
			extended_instr.set_non_negative_flag(true);
		}

		let offset_loaded_value = if matches!(value_offset, 0) {
			extended_loaded_value
		} else {
			let offset_value = i32_type.const_int(value_offset as u64, false);

			self.builder.build_int_add(
				extended_loaded_value,
				offset_value,
				"output_current_cell_add",
			)?
		};

		let putchar_call = self.builder.build_call(
			self.functions.putchar,
			&[offset_loaded_value.into()],
			"output_current_cell_call",
		)?;

		putchar_call.set_tail_call(true);

		let putchar_value = putchar_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		self.add_range_io_metadata(putchar_value, u8::MIN.into(), u8::MAX.into())?;

		Ok(())
	}

	fn output_char(&self, c: u8) -> Result<(), LlvmAssemblyError> {
		let char_to_put = {
			let i32_type = self.context().i32_type();

			i32_type.const_int(c.into(), false)
		};

		let putchar_call = self.builder.build_call(
			self.functions.putchar,
			&[char_to_put.into()],
			"output_char_call",
		)?;

		putchar_call.set_tail_call(true);

		let putchar_value = putchar_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		self.add_range_io_metadata(putchar_value, c.into(), c.into())?;

		Ok(())
	}

	fn output_chars(&self, c: &[u8]) -> Result<(), LlvmAssemblyError> {
		let constant_initializer = self.context().const_string(c, true);

		let constant_s_ty = constant_initializer.get_type();

		let global_constant = self.module.add_global(constant_s_ty, None, "output_chars");

		global_constant.set_initializer(&constant_initializer);
		global_constant.set_constant(true);

		let puts_call = self.builder.build_call(
			self.functions.puts,
			&[global_constant.as_pointer_value().into()],
			"output_chars_call",
		)?;

		puts_call.set_tail_call(true);

		let puts_value = puts_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		let last = c.last().copied().unwrap();

		self.add_range_io_metadata(puts_value, last.into(), last.into())?;

		let byref_attribute = self.context().create_type_attribute(
			Attribute::get_named_enum_kind_id("byref"),
			constant_s_ty.into(),
		);

		puts_call.add_attribute(AttributeLoc::Param(0), byref_attribute);

		let dereferenceable_attribute = self.context().create_enum_attribute(
			Attribute::get_named_enum_kind_id("dereferenceable"),
			c.len() as u64 + 1,
		);

		puts_call.add_attribute(AttributeLoc::Param(0), dereferenceable_attribute);

		Ok(())
	}

	fn add_range_io_metadata(
		&self,
		char_call: IntValue<'ctx>,
		min_inclusive: u64,
		max_inclusive: u64,
	) -> Result<(), InstructionValueError> {
		let Some(char_instr) = char_call.as_instruction() else {
			return Ok(());
		};

		let i32_type = self.context().i32_type();

		let i32_i8_min = i32_type.const_int(min_inclusive, false);

		let i32_i8_max = i32_type.const_int(max_inclusive + 1, false);

		let range_metadata_id = self.context().get_kind_id("range");

		let range_metadata_node = self
			.context()
			.metadata_node(&[i32_i8_min.into(), i32_i8_max.into()]);

		char_instr.set_metadata(range_metadata_node, range_metadata_id)
	}

	pub fn input_into_cell(&self) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		let getchar_call =
			self.builder
				.build_call(self.functions.getchar, &[], "input_into_cell_call")?;

		getchar_call.set_tail_call(true);

		let getchar_value = getchar_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		self.add_range_io_metadata(getchar_value, u8::MIN.into(), u8::MAX.into())?;

		let truncated_value =
			self.builder
				.build_int_truncate(getchar_value, i8_type, "input_into_cell_truncate")?;

		self.store(truncated_value, 0, "input_into_cell")
	}
}
