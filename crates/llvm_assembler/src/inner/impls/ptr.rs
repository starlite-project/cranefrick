use inkwell::values::IntValue;

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn move_pointer(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let ptr = self.load_ptr(0)?;

		let offset = {
			let i64_type = self.context.i64_type();

			i64_type.const_int(offset as u64, false)
		};

		let ptr = self.builder.build_int_add(ptr, offset, "offset ptr")?;

		self.builder.build_store(self.ptr, ptr)?;

		Ok(())
	}

	pub fn load_ptr(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let load = self
			.builder
			.build_load(self.context.i64_type(), self.ptr, "load ptr")?
			.into_int_value();

		let offset = {
			let i64_type = self.context.i64_type();

			i64_type.const_int(offset as u64, false)
		};

		let ptr = self.builder.build_int_add(load, offset, "offset ptr")?;

		Ok(ptr)
	}
}
