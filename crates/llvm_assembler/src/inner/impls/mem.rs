use inkwell::values::IntValue;

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let i8_type = self.context.i8_type();
		let i8_array_type = i8_type.array_type(30_000);
		let ptr_int_type = self.ptr_type;

		let current_offset = self.offset_ptr(offset)?;

		let zero = ptr_int_type.const_zero();

		let value = unsafe {
			self.builder.build_in_bounds_gep(
				i8_array_type,
				self.tape,
				&[zero, current_offset],
				"load_gep",
			)
		}?;

		let loaded_value = self
			.builder
			.build_load(i8_type, value, "load_load")?
			.into_int_value();

		if let Some(instr) = loaded_value.as_instruction() {
			let noundef_metadata_id = self.context.get_kind_id("noundef");
			let noalias_metadata_id = self.context.get_kind_id("noalias");
			let empty_metadata_node = self.context.metadata_node(&[]);

			instr
				.set_metadata(empty_metadata_node, noundef_metadata_id)
				.map_err(|_| LlvmAssemblyError::InvalidMetadata)?;

			instr
				.set_metadata(empty_metadata_node, noalias_metadata_id)
				.map_err(|_| LlvmAssemblyError::InvalidMetadata)?;
		}

		Ok(loaded_value)
	}

	pub fn store(&self, value: IntValue<'ctx>, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context.i8_type();
		let i8_array_type = i8_type.array_type(30_000);
		let ptr_int_type = self.ptr_type;

		let current_offset = self.offset_ptr(offset)?;

		let zero = ptr_int_type.const_zero();

		let current_tape_value = unsafe {
			self.builder.build_in_bounds_gep(
				i8_array_type,
				self.tape,
				&[zero, current_offset],
				"store_gep",
			)
		}?;

		let instr = self.builder.build_store(current_tape_value, value)?;

		let noalias_metadata_id = self.context.get_kind_id("noalias");

		let empty_metadata_node = self.context.metadata_node(&[]);

		instr
			.set_metadata(empty_metadata_node, noalias_metadata_id)
			.map_err(|_| LlvmAssemblyError::InvalidMetadata)?;

		Ok(())
	}
}
