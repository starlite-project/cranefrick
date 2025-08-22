use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn output_current_cell(&self) -> Result<(), LlvmAssemblyError> {
		let char_to_put = self.load(0)?;

		self.builder.build_call(
			self.functions.putchar,
			&[char_to_put.into()],
			"call putchar with current cell",
		)?;

		Ok(())
	}

	pub fn output_char(&self, c: u8) -> Result<(), LlvmAssemblyError> {
		let char_to_put = {
			let i8_type = self.context.i8_type();

			i8_type.const_int(c.into(), false)
		};

		self.builder.build_call(
			self.functions.putchar,
			&[char_to_put.into()],
			&format!("call putchar with char {c}"),
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
				"index_into_tape_for_write",
			)?
		};

		self.builder
			.build_call(self.functions.getchar, &[ptr_value.into()], "call_putchar")?;

		Ok(())
	}
}
