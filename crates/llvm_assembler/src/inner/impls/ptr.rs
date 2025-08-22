use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn move_pointer(&mut self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i64_type = self.context.i64_type();
		let offset = i64_type.const_int(offset as u64, false);

		self.ptr = self.builder.build_int_add(self.ptr, offset, "offset ptr")?;

		Ok(())
	}
}
