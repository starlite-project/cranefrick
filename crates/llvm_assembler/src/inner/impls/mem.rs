use inkwell::values::IntValue;

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let i8_type = self.context.i8_type();

		let current_offset = self.offset_ptr(offset)?;

		let value = unsafe {
			self.builder
				.build_in_bounds_gep(i8_type, self.tape, &[current_offset], "load_gep")
		}?;

		let loaded_value = self
			.builder
			.build_load(i8_type, value, "load_load")?
			.into_int_value();

		if let Some(instr) = loaded_value.as_instruction() {
			let noalias_metadata_id = self.context.get_kind_id("noalias");

			let noalias_metadata_node = self.context.metadata_node(&[]);

			instr
				.set_metadata(noalias_metadata_node, noalias_metadata_id)
				.unwrap();
		}

		Ok(loaded_value)
	}

	pub fn store(&self, value: IntValue<'ctx>, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context.i8_type();

		let current_offset = self.offset_ptr(offset)?;

		let current_tape_value = unsafe {
			self.builder
				.build_in_bounds_gep(i8_type, self.tape, &[current_offset], "store_gep")
		}?;

		let instr = self.builder.build_store(current_tape_value, value)?;

		let noalias_metadata_id = self.context.get_kind_id("noalias");

		let noalias_metadata_node = self.context.metadata_node(&[]);

		instr
			.set_metadata(noalias_metadata_node, noalias_metadata_id)
			.unwrap();

		Ok(())
	}
}
