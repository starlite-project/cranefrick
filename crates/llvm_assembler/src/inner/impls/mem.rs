use inkwell::values::IntValue;

use crate::{ContextExt, LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let ptr = self.load_ptr(offset)?;

		let i8_type = self.context.i8_type();

		let current_index_ptr = unsafe {
			self.builder
				.build_gep(i8_type, self.tape, &[ptr], "load array index ptr")?
		};

		let value = self
			.builder
			.build_load(i8_type, current_index_ptr, "load array value")?;

		Ok(value.into_int_value())
	}

	pub fn store(&self, value: IntValue<'ctx>, offset: i32) -> Result<(), LlvmAssemblyError> {
		let ptr = self.load_ptr(offset)?;

		let i8_type = self.context.i8_type();

		let current_index_ptr = unsafe {
			self.builder
				.build_gep(i8_type, self.tape, &[ptr], "load array index ptr")?
		};

		self.builder.build_store(current_index_ptr, value)?;

		Ok(())
	}
}
