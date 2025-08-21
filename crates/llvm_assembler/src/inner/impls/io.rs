use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn output_current_cell(&self) -> Result<(), LlvmAssemblyError> {
		let char_to_put = self.load(0)?;

		self.builder.build_call(
			self.functions.putchar,
			&[char_to_put.into()],
			"call putchar",
		)?;

		Ok(())
	}
}
