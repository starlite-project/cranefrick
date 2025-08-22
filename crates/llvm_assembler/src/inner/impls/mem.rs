use inkwell::values::IntValue;

use crate::{ContextExt as _, LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let ptr_type = self.context.default_ptr_type();
		let i8_type = self.context.i8_type();
		let i32_type = self.context.i32_type();

		let offset = i32_type.const_int(offset as u64, false);

		let current_offset = {
			let current_ptr = self
				.builder
				.build_load(i32_type, self.ptr, "load ptr")?
				.into_int_value();

			self.builder
				.build_int_add(current_ptr, offset, "offset ptr for load")
		}?;

		let value = unsafe {
			self.builder
				.build_in_bounds_gep(ptr_type, self.tape, &[current_offset], "index into tape")
		}?;

		let loaded_value = self
			.builder
			.build_load(i8_type, value, "load value from tape")?;

		Ok(loaded_value.into_int_value())
	}

	pub fn store(&self, value: IntValue<'ctx>, offset: i32) -> Result<(), LlvmAssemblyError> {
		let ptr_type = self.context.default_ptr_type();
		let i32_type = self.context.i32_type();

		let offset = i32_type.const_int(offset as u64, false);

		let current_offset = {
			let current_ptr = self
				.builder
				.build_load(i32_type, self.ptr, "load ptr")?
				.into_int_value();

			self.builder
				.build_int_add(current_ptr, offset, "offset ptr for store")
		}?;

		let current_tape_value = unsafe {
			self.builder
				.build_in_bounds_gep(ptr_type, self.tape, &[current_offset], "index into tape")
		}?;

		self.builder.build_store(current_tape_value, value)?;

		Ok(())
	}
}
