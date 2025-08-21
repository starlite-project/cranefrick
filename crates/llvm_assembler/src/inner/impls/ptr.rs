use crate::{ContextExt, LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn move_pointer(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i64_type = self.context.i64_type();
		let ptr_type = self.context.default_ptr_type();
		let offset = i64_type.const_int(offset as u64, false);

		let ptr_load = self
			.builder
			.build_load(ptr_type, self.ptr, "load ptr")?
			.into_pointer_value();

		let result = unsafe {
			self.builder
				.build_in_bounds_gep(ptr_type, ptr_load, &[offset], "offset pointer")?
		};

		self.builder.build_store(self.ptr, result)?;

		Ok(())
	}
}
