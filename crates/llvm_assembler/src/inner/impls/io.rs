use crate::{ContextExt, LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn output_char(&self, c: u8) -> Result<(), LlvmAssemblyError> {
		let write = self.functions.putchar;

		let value = {
			let i8_type = self.context.i8_type();

			i8_type.const_int(c.into(), false)
		};

		self.builder.build_call(
			write,
			&[value.into()],
			&format!("output char {}", c as char),
		)?;

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

	pub fn input_into_cell(&self) -> Result<(), LlvmAssemblyError> {
		let ptr = self.load_ptr(0)?;

		let current_index_ptr = {
			let i64_type = self.context.i64_type();
			let ptr_type = self.context.default_ptr_type();

			let tape_ptr = self.tape.const_to_int(i64_type);

			let tape_offset_ptr = self
				.builder
				.build_int_add(tape_ptr, ptr, "offset tape ptr")?;

			self.builder
				.build_int_to_ptr(tape_offset_ptr, ptr_type, "cast int to ptr")?
		};

		let write = self.functions.getchar;

		self.builder
			.build_call(write, &[current_index_ptr.into()], "call getchar")?;

		Ok(())
	}
}
