use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn output_char(&self, c: u8) -> Result<(), LlvmAssemblyError>{
		let write = self.functions.putchar;

		let value = {
			let i8_type = self.context.i8_type();

			i8_type.const_int(c.into(), false)
		};

		self.builder.build_call(write, &[value.into()], &format!("output char {}", c as char))?;

		Ok(())
	}

	pub fn output_chars(&self, c: &[u8]) -> Result<(), LlvmAssemblyError> {
		c.iter().copied().try_for_each(|c| self.output_char(c))
	}

	pub fn output_current_cell(&self) -> Result<(), LlvmAssemblyError> {
		let value = self.load(0)?;

		let write = self.functions.putchar;

		self.builder
			.build_call(write, &[value.into()], "output current char")?;

		Ok(())
	}
}
