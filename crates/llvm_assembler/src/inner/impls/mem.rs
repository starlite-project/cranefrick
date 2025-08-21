use inkwell::values::IntValue;

use crate::{ContextExt, LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let ptr = self.load_ptr(offset)?;

		let i8_type = self.context.i8_type();

		let current_index_ptr = {
			let i64_type = self.context.i64_type();
			let ptr_type = self.context.default_ptr_type();

			let tape_ptr = self.tape.const_to_int(i64_type);

			let tape_offset_ptr = self
				.builder
				.build_int_add(tape_ptr, ptr, "offset tape ptr")?;

			self.builder
				.build_int_to_ptr(tape_offset_ptr, ptr_type, "cast int to ptr")?
		};

		let value = self
			.builder
			.build_load(i8_type, current_index_ptr, "load array value")?;

		Ok(value.into_int_value())
	}

	pub fn store(&self, value: IntValue<'ctx>, offset: i32) -> Result<(), LlvmAssemblyError> {
		let ptr = self.load_ptr(offset)?;

		let current_index_ptr = {
			let i64_type = self.context.i64_type();
			let ptr_type = self.context.default_ptr_type();

			let tape_ptr = self.tape.const_to_int(i64_type);

			let tape_offset_ptr = self.builder.build_int_add(tape_ptr, ptr, "offset tape ptr")?;

			self.builder
				.build_int_to_ptr(tape_offset_ptr, ptr_type, "cast int to ptr")?
		};

		self.builder.build_store(current_index_ptr, value)?;

		Ok(())
	}
}
