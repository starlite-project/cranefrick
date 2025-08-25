use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn output_current_cell(&self) -> Result<(), LlvmAssemblyError> {
		let char_to_put = self.load(0)?;

		let i32_type = self.context.i32_type();

		let extended_char =
			self.builder
				.build_int_z_extend(char_to_put, i32_type, "output_current_cell_extend")?;

		self.builder.build_call(
			self.functions.putchar,
			&[extended_char.into()],
			"output_current_cell_call",
		)?;

		Ok(())
	}

	pub fn output_char(&self, c: u8) -> Result<(), LlvmAssemblyError> {
		let char_to_put = {
			let i8_type = self.context.i8_type();

			i8_type.const_int(c.into(), false)
		};

		let i32_type = self.context.i32_type();

		let extended_char =
			self.builder
				.build_int_z_extend(char_to_put, i32_type, "output_char_extend")?;

		self.builder.build_call(
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
		let i8_type = self.context.i8_type();
		let i8_array_type = i8_type.array_type(30_000);

		let current_ptr = self.offset_ptr(0)?;

		let zero = self.ptr_type.const_zero();

		let ptr_value = unsafe {
			self.builder.build_gep(
				i8_array_type,
				self.tape,
				&[zero, current_ptr],
				"input_into_cell_gep",
			)?
		};

		self.builder.build_call(
			self.functions.getchar,
			&[ptr_value.into()],
			"input_into_cell_call",
		)?;

		Ok(())
	}
}
