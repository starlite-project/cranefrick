use frick_assembler::TAPE_SIZE;
use inkwell::{IntPredicate, values::IntValue};

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn move_pointer(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let wrapped_ptr = self.offset_ptr(offset)?;

		self.builder.build_store(self.ptr, wrapped_ptr)?;

		Ok(())
	}

	pub fn offset_ptr(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let ptr_type = self.ptr_int_type;
		let offset_value = ptr_type.const_int(offset as u64, false);

		let current_ptr = self
			.builder
			.build_load(ptr_type, self.ptr, "offset_ptr_load")?
			.into_int_value();

		if let Some(instr) = current_ptr.as_instruction() {
			let range_metadata_id = self.context.get_kind_id("range");
			let range_metadata_node = self.context.metadata_node(&[
				ptr_type.const_zero().into(),
				ptr_type.const_int(TAPE_SIZE as u64, false).into(),
			]);

			instr
				.set_metadata(range_metadata_node, range_metadata_id)
				.map_err(|_| LlvmAssemblyError::InvalidMetadata)?;
		}

		if matches!(offset, 0) {
			return Ok(current_ptr);
		}

		let offset_ptr = self
			.builder
			.build_int_add(current_ptr, offset_value, "offset_ptr_add")?;

		let wrapped_offset_ptr = if offset > 0 {
			self.wrap_ptr_positive(offset_ptr)
		} else {
			self.wrap_ptr_negative(offset_ptr)
		}?;

		Ok(wrapped_offset_ptr)
	}

	fn wrap_ptr_positive(
		&self,
		offset_ptr: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let ptr_int_type = self.ptr_int_type;

		let ptr_offset = self.builder.build_int_unsigned_rem(
			offset_ptr,
			ptr_int_type.const_int(TAPE_SIZE as u64, false),
			"wrap_ptr_positive_urem",
		)?;

		Ok(ptr_offset)
	}

	fn wrap_ptr_negative(
		&self,
		offset_ptr: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let ptr_int_type = self.ptr_int_type;

		let tape_size = ptr_int_type.const_int(TAPE_SIZE as u64, false);

		let tmp =
			self.builder
				.build_int_signed_rem(offset_ptr, tape_size, "wrap_ptr_negative_srem")?;

		let added_offset = self
			.builder
			.build_int_add(tmp, tape_size, "wrap_ptr_negative_add")?;

		let cmp = self.builder.build_int_compare(
			IntPredicate::SLT,
			tmp,
			ptr_int_type.const_zero(),
			"wrap_ptr_negative_cmp",
		)?;

		let ptr_offset = self
			.builder
			.build_select(cmp, added_offset, tmp, "wrap_ptr_negative_select")?
			.into_int_value();

		Ok(ptr_offset)
	}
}
