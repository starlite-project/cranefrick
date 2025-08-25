use inkwell::values::IntValue;

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let i8_type = self.context.i8_type();
		let i8_array_type = i8_type.array_type(30_000);

		let current_offset = self.offset_ptr(offset)?;

		let zero = self.ptr_type.const_zero();

		let value = unsafe {
			self.builder.build_in_bounds_gep(
				i8_array_type,
				self.tape,
				&[zero, current_offset],
				"load_gep",
			)
		}?;

		let loaded_value = self.builder.build_load(i8_type, value, "load_load")?;

		Ok(loaded_value.into_int_value())
	}

	pub fn store(&self, value: IntValue<'ctx>, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context.i8_type();
		let i8_array_type = i8_type.array_type(30_000);

		let current_offset = self.offset_ptr(offset)?;

		let zero = self.ptr_type.const_zero();

		let current_tape_value = unsafe {
			self.builder.build_in_bounds_gep(
				i8_array_type,
				self.tape,
				&[zero, current_offset],
				"store_gep",
			)
		}?;

		self.builder.build_store(current_tape_value, value)?;

		Ok(())
	}
}
