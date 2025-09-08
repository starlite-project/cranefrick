use std::ops::RangeInclusive;

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn set_range(
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

		let values_to_set = {
			let range_len_value = {
				let ptr_int_type = self.ptr_type;

				ptr_int_type.const_int(range_len as u64, false)
			};

			let array_alloca =
				self.builder
					.build_array_alloca(i8_type, range_len_value, "set_range_alloca")?;

			self.builder.build_memset(
				array_alloca,
				1,
				i8_type.const_int(value.into(), false),
				range_len_value,
			)?;

			self.builder.build_load(
				i8_type.array_type(range_len as u32),
				array_alloca,
				"set_range_load",
			)?
		}
		.into_array_value();

		self.builder.build_store(gep, values_to_set)?;

		Ok(())
	}
}
