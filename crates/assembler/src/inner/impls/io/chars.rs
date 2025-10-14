use crate::{AssemblyError, ContextGetter as _, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub(super) fn output_char(&self, c: u8) -> Result<(), AssemblyError> {
		let context = self.context();

		let char_value = {
			let i32_type = context.i32_type();

			i32_type.const_int(c.into(), false)
		};

		self.call_putchar(context, char_value, "output_char")
	}

	pub(super) fn output_str(&self, c: &[u8]) -> Result<(), AssemblyError> {
		let context = self.context();

		let constant_string = context.const_string(c, false);

		let constant_string_ty = constant_string.get_type();

		let global_constant =
			self.module
				.add_global(constant_string_ty, None, "output_str_global_value\0");

		self.setup_global_value(global_constant, &constant_string);

		let global_value_pointer = global_constant.as_pointer_value();

		self.call_puts(context, global_value_pointer, c.len() as u64, "output_str")
	}
}
