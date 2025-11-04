use frick_utils::Convert as _;

use crate::{AssemblyError, ContextGetter as _, inner::InnerAssembler};

impl InnerAssembler<'_> {
	#[tracing::instrument(skip_all)]
	pub(super) fn output_char(&self, c: u8) -> Result<(), AssemblyError> {
		let context = self.context();

		let char_value = {
			let i8_type = context.i8_type();

			i8_type.const_int(c.convert::<u64>(), false)
		};

		self.call_putchar(context, char_value)
	}

	#[tracing::instrument(skip_all)]
	pub(super) fn output_str(&self, c: &[u8]) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();

		c.iter()
			.map(|&x| i8_type.const_int(x.convert::<u64>(), false))
			.try_for_each(|x| self.call_putchar(context, x))
	}
}
