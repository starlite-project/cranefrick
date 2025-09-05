use std::ops::RangeInclusive;

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn set_range(
		&self,
		value: u8,
		range: RangeInclusive<i32>,
	) -> Result<(), LlvmAssemblyError> {
		let range_len = range.clone().count();
		let i8_type = self.context.i8_type();

		let start = *range.start();

		let current_offset = self.offset_ptr(start)?;

		let gep = unsafe {
			self.builder
				.build_in_bounds_gep(i8_type, self.tape, &[current_offset], "set_range_gep")
		}?;

		let values_to_set = {
			let array_alloca = self
				.builder
				// .build_alloca(i8_type.array_type(range_len as u32), "set_range_alloca")?;
				.build_array_alloca(i8_type, self.ptr_type.const_int(range_len as u64, false), "set_range_alloca")?;

			self.builder.build_memset(
				array_alloca,
				1,
				i8_type.const_int(value.into(), false),
				self.ptr_type.const_int(range_len as u64, false),
			)?;

			self.builder.build_load(
				i8_type.array_type(range_len as u32),
				array_alloca,
				"set_range_load",
			)
		}?;

		self.builder.build_store(gep, values_to_set)?;

		Ok(())
	}
}
