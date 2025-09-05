use std::ops::RangeInclusive;

use inkwell::types::VectorType;

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

		let vector_to_set = {
			let vec_of_values = std::iter::repeat_n(value, range_len);

			let vec_of_llvm_values = vec_of_values
				.map(|v| i8_type.const_int(v.into(), false))
				.collect::<Vec<_>>();

			VectorType::const_vector(&vec_of_llvm_values)
		};

		self.builder.build_store(gep, vector_to_set)?;

		Ok(())
	}
}
