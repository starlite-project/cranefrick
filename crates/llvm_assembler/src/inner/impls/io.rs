use frick_assembler::AssemblyError;
use frick_ir::{BrainIr, OutputOptions};

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn output(&self, options: &OutputOptions) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		match options {
			OutputOptions::Cell(options) => {
				self.output_current_cell(options.value(), options.offset())?;
			}
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

	fn output_current_cell(&self, value_offset: i8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i32_type = self.context().i32_type();
		let loaded_value = self.load(offset, "output_current_cell")?;

		let extended_loaded_value = self.builder.build_int_z_extend(
			loaded_value,
			i32_type,
			"output_current_cell_extend",
		)?;

		let offset_loaded_value = {
			let offset_value = i32_type.const_int(value_offset as u64, false);

			self.builder.build_int_add(
				extended_loaded_value,
				offset_value,
				"output_current_cell_add",
			)?
		};

		self.builder.build_call(
			self.functions.putchar,
			&[offset_loaded_value.into()],
			"output_current_cell_call",
		)?;

		Ok(())
	}

	fn output_char(&self, c: u8) -> Result<(), LlvmAssemblyError> {
		let char_to_put = {
			let i32_type = self.context().i32_type();

			i32_type.const_int(c.into(), false)
		};

		self.builder.build_call(
			self.functions.putchar,
			&[char_to_put.into()],
			"output_char_call",
		)?;

		Ok(())
	}

	fn output_chars(&self, c: &[u8]) -> Result<(), LlvmAssemblyError> {
		c.iter().copied().try_for_each(|c| self.output_char(c))?;

		Ok(())
	}

	pub fn input_into_cell(&self) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		self.builder.build_call(
			self.functions.getchar,
			&[self.pointers.input.into()],
			"input_into_cell_call",
		)?;

		let gep = {
			let current_ptr = self.offset_ptr(0)?;

			self.gep(i8_type, current_ptr, "input_into_cell")?
		};

		let i8_size = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(1, false)
		};

		self.builder
			.build_memcpy(gep, 1, self.pointers.input, 1, i8_size)?;

		Ok(())
	}
}
