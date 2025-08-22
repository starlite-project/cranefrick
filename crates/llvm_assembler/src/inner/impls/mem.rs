use inkwell::values::IntValue;

use crate::{ContextExt as _, LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let ptr_type = self.context.default_ptr_type();
		let i8_type = self.context.i8_type();

		let current_offset = self.offset_ptr(offset)?;

		let value = unsafe {
			self.builder
				.build_gep(ptr_type, self.tape, &[current_offset], "index_into_tape")
		}?;

		let loaded_value = self
			.builder
			.build_load(i8_type, value, "load_value_from_tape")?;

		Ok(loaded_value.into_int_value())
	}

	pub fn store(&self, value: IntValue<'ctx>, offset: i32) -> Result<(), LlvmAssemblyError> {
		let ptr_type = self.context.default_ptr_type();

		let current_offset = self.offset_ptr(offset)?;

		let current_tape_value = unsafe {
			self.builder
				.build_gep(ptr_type, self.tape, &[current_offset], "index_into_tape")
		}?;

		self.builder.build_store(current_tape_value, value)?;

		Ok(())
	}
}
