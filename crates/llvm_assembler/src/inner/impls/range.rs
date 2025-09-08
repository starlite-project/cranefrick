use std::ops::RangeInclusive;

use frick_assembler::TAPE_SIZE;

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn mem_set(
		&self,
		value: u8,
		range: RangeInclusive<i32>,
	) -> Result<(), LlvmAssemblyError> {
		let start = *range.start();
		let range_len = range.count();
		let i8_type = self.context.i8_type();

		let current_offset = self.offset_ptr(start)?;

		let gep = unsafe {
			self.builder
				.build_in_bounds_gep(i8_type, self.tape, &[current_offset], "set_range_gep")
		}?;

		let range_len_value = {
			let ptr_int_type = self.ptr_int_type;

			ptr_int_type.const_int(range_len as u64, false)
		};

		let value_value = i8_type.const_int(value.into(), false);

		self.builder
			.build_memset(gep, 1, value_value, range_len_value)?;

		Ok(())
	}
}
