use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn output_current_cell(
		&self,
		cell_offset: i8,
		offset: i32,
	) -> Result<(), LlvmAssemblyError> {
		let i32_type = self.context().i32_type();
		let char_to_put = self.load(offset, "output_current_cell")?;

		let cell_offset_value = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(cell_offset as u64, false)
		};

		let offset_char = self.builder.build_int_add(
			char_to_put,
			cell_offset_value,
			"output_current_cell_add",
		)?;

		let extended_char =
			self.builder
				.build_int_z_extend(offset_char, i32_type, "output_current_cell_extend")?;

		self.builder.build_call(
			self.functions.putchar,
			&[extended_char.into()],
			"output_current_cell_call",
		)?;

		Ok(())
	}

	pub fn output_char(&self, c: u8) -> Result<(), LlvmAssemblyError> {
		let char_to_put = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(c.into(), false)
		};

		let i32_type = self.context().i32_type();

		let extended_char =
			self.builder
				.build_int_z_extend(char_to_put, i32_type, "output_char_extend")?;

		self.builder.build_direct_call(
			self.functions.putchar,
			&[extended_char.into()],
			"output_char_call",
		)?;

		Ok(())
	}

	pub fn output_chars(&self, c: &[u8]) -> Result<(), LlvmAssemblyError> {
		c.iter().copied().try_for_each(|c| self.output_char(c))
	}

	pub fn input_into_cell(&self) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		let current_ptr = self.offset_ptr(0)?;

		let gep = self.gep(i8_type, current_ptr, "input_into_cell")?;

		self.builder.build_direct_call(
			self.functions.getchar,
			&[gep.into()],
			"input_into_cell_call",
		)?;

		Ok(())
	}
}
