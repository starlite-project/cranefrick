use frick_spec::TAPE_SIZE;
use inkwell::{IntPredicate, values::IntValue};

use crate::{
	AssemblyError,
	inner::{InnerAssembler, utils::CalculatedOffset},
};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn move_pointer(&self, offset: i32) -> Result<(), AssemblyError> {
		let wrapped_ptr = self.offset_pointer(offset)?;

		self.builder
			.build_store(self.pointers.pointer, wrapped_ptr)?;

		Ok(())
	}

	pub fn resolve_offset(
		&self,
		offset: CalculatedOffset<'ctx>,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		match offset {
			CalculatedOffset::Calculated(offset) => Ok(offset),
			CalculatedOffset::Raw(offset) => self.offset_pointer(offset),
		}
	}

	pub fn offset_pointer(&self, offset: i32) -> Result<IntValue<'ctx>, AssemblyError> {
		let ptr_type = self.ptr_int_type;
		let offset_value = ptr_type.const_int(offset as u64, false);

		let current_ptr = self
			.builder
			.build_load(ptr_type, self.pointers.pointer, "offset_pointer_load")?
			.into_int_value();

		let new_ptr_value = if matches!(offset, 0) {
			Ok(current_ptr)
		} else {
			let offset_ptr =
				self.builder
					.build_int_add(current_ptr, offset_value, "offset_pointer_add")?;

			if offset > 0 {
				self.wrap_pointer_positive(offset_ptr)
			} else {
				self.wrap_pointer_negative(offset_ptr)
			}
		}?;

		if let Some(new_ptr_instr) = new_ptr_value.as_instruction() {
			self.debug_builder.insert_dbg_value_before(
				new_ptr_value.into(),
				self.debug_builder.variables.pointer,
				None,
				self.builder.get_current_debug_location().unwrap(),
				new_ptr_instr,
			);
		}

		Ok(new_ptr_value)
	}

	fn wrap_pointer_positive(
		&self,
		offset_ptr: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		let ptr_int_type = self.ptr_int_type;

		Ok(self.builder.build_int_unsigned_rem(
			offset_ptr,
			ptr_int_type.const_int(TAPE_SIZE as u64, false),
			"wrap_pointer_positive_urem",
		)?)
	}

	fn wrap_pointer_negative(
		&self,
		offset_ptr: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		let ptr_int_type = self.ptr_int_type;

		let tape_size = ptr_int_type.const_int(TAPE_SIZE as u64, false);

		let tmp = self.builder.build_int_signed_rem(
			offset_ptr,
			tape_size,
			"wrap_pointer_negative_srem",
		)?;

		let added_offset =
			self.builder
				.build_int_add(tmp, tape_size, "wrap_pointer_negative_add")?;

		let cmp = self.builder.build_int_compare(
			IntPredicate::SLT,
			tmp,
			ptr_int_type.const_zero(),
			"wrap_pointer_negative_cmp",
		)?;

		Ok(self
			.builder
			.build_select(cmp, added_offset, tmp, "wrap_pointer_negative_select")
			.map(|i| i.into_int_value())?)
	}
}
