use inkwell::values::IntValue;

use crate::{ContextExt, LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let ptr_type = self.context.default_ptr_type();
		let i8_type = self.context.i8_type();

		let offset = {
			let i64_type = self.context.i64_type();

			i64_type.const_int(offset as u64, false)
		};

		let result_ptr = unsafe {
			self.builder
				.build_in_bounds_gep(ptr_type, self.ptr, &[offset], "offset ptr")?
		};

		let load = self.builder.build_load(i8_type, result_ptr, "load ptr")?;

		Ok(load.into_int_value())
	}

	pub fn store(&self, value: IntValue<'ctx>, offset: i32) -> Result<(), LlvmAssemblyError> {
		let ptr_type = self.context.default_ptr_type();

		let offset = {
			let i64_type = self.context.i64_type();

			i64_type.const_int(offset as u64, false)
		};

		let result_ptr = unsafe {
			self.builder
				.build_in_bounds_gep(ptr_type, self.ptr, &[offset], "offset ptr")?
		};

		self.builder.build_store(result_ptr, value)?;

		Ok(())
	}
}
