use frick_spec::TAPE_SIZE;
use inkwell::{IntPredicate, values::IntValue};

use crate::{
	AssemblyError,
	inner::{InnerAssembler, utils::CalculatedOffset},
};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip_all)]
	pub fn move_pointer(&self, offset: i32) -> Result<(), AssemblyError> {
		let wrapped_ptr = self.offset_pointer(offset)?;

		let store_instr = self
			.builder
			.build_store(self.pointers.pointer, wrapped_ptr)?;

		self.debug_builder.insert_dbg_value_before(
			wrapped_ptr.into(),
			self.debug_builder.variables.pointer,
			None,
			self.builder.get_current_debug_location().unwrap(),
			store_instr,
		);

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub fn resolve_offset(
		&self,
		offset: CalculatedOffset<'ctx>,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		match offset {
			CalculatedOffset::Calculated(offset) => Ok(offset),
			CalculatedOffset::Raw(offset) => self.offset_pointer(offset),
		}
	}

	#[tracing::instrument(skip_all)]
	pub fn offset_pointer(&self, offset: i32) -> Result<IntValue<'ctx>, AssemblyError> {
		let ptr_type = self.ptr_int_type;
		let offset_value = ptr_type.const_int(offset as u64, false);

		let current_ptr = self
			.builder
			.build_load(ptr_type, self.pointers.pointer, "offset_pointer_load\0")?
			.into_int_value();

		if matches!(offset, 0) {
			Ok(current_ptr)
		} else {
			let offset_ptr = self.builder.build_int_nsw_add(
				current_ptr,
				offset_value,
				"offset_pointer_add\0",
			)?;

			self.wrap_pointer(offset_ptr, offset > 0)
		}
	}

	#[tracing::instrument(skip_all)]
	pub fn wrap_pointer(
		&self,
		offset_ptr: IntValue<'ctx>,
		is_positive: bool,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		if is_positive {
			self.wrap_pointer_positive(offset_ptr)
		} else {
			self.wrap_pointer_negative(offset_ptr)
		}
	}

	#[tracing::instrument(skip_all)]
	fn wrap_pointer_positive(
		&self,
		offset_ptr: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		let ptr_int_type = self.ptr_int_type;

		let tape_size = ptr_int_type.const_int(TAPE_SIZE as u64, false);

		Ok(self.builder.build_int_unsigned_rem(
			offset_ptr,
			tape_size,
			"wrap_pointer_positive_urem\0",
		)?)
	}

	#[tracing::instrument(skip_all)]
	fn wrap_pointer_negative(
		&self,
		offset_ptr: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		let ptr_int_type = self.ptr_int_type;

		let tape_size = ptr_int_type.const_int(TAPE_SIZE as u64, false);

		let tmp = self.builder.build_int_signed_rem(
			offset_ptr,
			tape_size,
			"wrap_pointer_negative_srem\0",
		)?;

		let added_offset =
			self.builder
				.build_int_nsw_add(tmp, tape_size, "wrap_pointer_negative_add\0")?;

		let cmp = self.builder.build_int_compare(
			IntPredicate::SLT,
			tmp,
			ptr_int_type.const_zero(),
			"wrap_pointer_negative_cmp\0",
		)?;

		Ok(self
			.builder
			.build_select(cmp, added_offset, tmp, "wrap_pointer_negative_select\0")
			.map(|i| i.into_int_value())?)
	}
}
