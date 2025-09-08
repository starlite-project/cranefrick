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

	pub fn change_range(
		&self,
		value: i8,
		range: RangeInclusive<i32>,
	) -> Result<(), LlvmAssemblyError> {
		let start = *range.start();
		let range_len = range.count();

		let i8_type = self.context.i8_type();
		let i8_vector_type = i8_type.vec_type(range_len as u32);
		let i8_array_type = i8_type.array_type(TAPE_SIZE as u32);

		let undef_vector = i8_vector_type.get_undef();
		let tmp_vector = self.builder.build_insert_element(
			undef_vector,
			i8_type.const_int(value as u64, false),
			i8_type.const_zero(),
			"change_range_insertelement",
		)?;

		let vector_of_values = self.builder.build_shuffle_vector(
			tmp_vector,
			i8_vector_type.get_undef(),
			i8_vector_type.const_zero(),
			"change_range_shufflevector",
		)?;

		let current_offset = self.offset_ptr(start)?;

		let zero = {
			let ptr_int_type = self.ptr_int_type;

			ptr_int_type.const_zero()
		};

		let gep = unsafe {
			self.builder.build_in_bounds_gep(
				i8_array_type,
				self.tape,
				&[zero, current_offset],
				"change_range_gep",
			)
		}?;

		let range_to_add_to = self
			.builder
			.build_load(i8_vector_type, gep, "change_range_load")?
			.into_vector_value();

		let added_range =
			self.builder
				.build_int_add(range_to_add_to, vector_of_values, "change_range_add")?;

		self.builder.build_store(gep, added_range)?;

		Ok(())
	}
}
