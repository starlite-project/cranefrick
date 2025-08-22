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
}
