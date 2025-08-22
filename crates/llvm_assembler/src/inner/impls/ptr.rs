use inkwell::{IntPredicate, values::IntValue};

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn move_pointer(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let wrapped_ptr = self.offset_ptr(offset)?;

		self.builder.build_store(self.ptr, wrapped_ptr)?;

		Ok(())
	}

	pub fn offset_ptr(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let ptr_type = self.ptr_type;
		let offset_value = ptr_type.const_int(offset as u64, false);

		let current_ptr = self
			.builder
			.build_load(ptr_type, self.ptr, "load_ptr")?
			.into_int_value();

		if matches!(offset, 0) {
			return Ok(current_ptr);
		}

		let offset_ptr = self
			.builder
			.build_int_add(current_ptr, offset_value, "offset_ptr")?;

		let wrapped_offset_ptr = if offset > 0 {
			self.builder.build_int_unsigned_rem(
				offset_ptr,
				ptr_type.const_int(30_000, false),
				"wrap_ptr_positive",
			)
		} else {
			let tmp = self.builder.build_int_signed_rem(
				offset_ptr,
				ptr_type.const_int(30_000, false),
				"tmp",
			)?;

			let added_offset =
				self.builder
					.build_int_add(tmp, ptr_type.const_int(30_000, false), "added_tmp")?;

			let cmp = self.builder.build_int_compare(
				IntPredicate::SLT,
				tmp,
				ptr_type.const_zero(),
				"isneg",
			)?;

			self.builder
				.build_select(cmp, added_offset, tmp, "wrapped")
				.map(|v| v.into_int_value())
		}?;

		Ok(wrapped_offset_ptr)
	}
}
