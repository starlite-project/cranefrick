use std::{fmt::Display, ops::RangeInclusive};

use frick_assembler::TAPE_SIZE;
use inkwell::{
	types::BasicType,
	values::{IntValue, PointerValue},
};

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(&self, offset: i32) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let i8_type = self.context.i8_type();

		let current_offset = self.offset_ptr(offset)?;

		let value = self.gep(i8_type, current_offset, "load")?;

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

		let current_offset = self.offset_ptr(offset)?;

		let current_tape_value = self.gep(i8_type, current_offset, "store")?;

		let instr = self.builder.build_store(current_tape_value, value)?;

		let noalias_metadata_id = self.context.get_kind_id("noalias");

		let empty_metadata_node = self.context.metadata_node(&[]);

		instr
			.set_metadata(empty_metadata_node, noalias_metadata_id)
			.map_err(|_| LlvmAssemblyError::InvalidMetadata)?;

		Ok(())
	}

	pub fn mem_set(&self, value: u8, range: RangeInclusive<i32>) -> Result<(), LlvmAssemblyError> {
		let start = *range.start();
		let range_len = range.count();
		let i8_type = self.context.i8_type();

		let current_offset = self.offset_ptr(start)?;

		let gep = self.gep(i8_type, current_offset, "set_range")?;

		let range_len_value = {
			let ptr_int_type = self.ptr_int_type;

			ptr_int_type.const_int(range_len as u64, false)
		};

		let value_value = i8_type.const_int(value.into(), false);

		self.builder
			.build_memset(gep, 1, value_value, range_len_value)?;

		Ok(())
	}

	pub fn mem_copy(&self, values: &[u8], start: i32) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context.i8_type();
		let i8_array_type = i8_type.array_type(values.len() as u32);
		let ptr_int_type = self.ptr_int_type;

		let array_len = ptr_int_type.const_int(values.len() as u64, false);

		let i8_array_alloca = self
			.builder
			.build_alloca(i8_array_type, "mem_copy_alloca")?;

		for (i, value) in values.iter().copied().enumerate() {
			let index = ptr_int_type.const_int(i as u64, false);

			let memcpy_array_gep = unsafe {
				self.builder
					.build_gep(i8_type, i8_array_alloca, &[index], "mem_copy_gep")
			}?;

			self.builder
				.build_store(memcpy_array_gep, i8_type.const_int(value.into(), false))?;
		}

		let current_offset = self.offset_ptr(start)?;

		let gep = self.gep(i8_type, current_offset, "mem_copy")?;

		self.builder
			.build_memcpy(gep, 1, i8_array_alloca, 1, array_len)?;

		Ok(())
	}

	pub fn gep<T>(
		&self,
		ty: T,
		offset: IntValue<'ctx>,
		name: impl Display,
	) -> Result<PointerValue<'ctx>, LlvmAssemblyError>
	where
		T: BasicType<'ctx>,
	{
		let basic_type = ty.as_basic_type_enum();

		let gep = if basic_type.is_array_type() {
			let zero = {
				let ptr_int_type = self.ptr_int_type;

				ptr_int_type.const_zero()
			};

			unsafe {
				self.builder.build_in_bounds_gep(
					basic_type.into_array_type(),
					self.tape,
					&[zero, offset],
					&format!("{name}_array_gep"),
				)?
			}
		} else {
			unsafe {
				self.builder.build_in_bounds_gep(
					basic_type.into_int_type(),
					self.tape,
					&[offset],
					&format!("{name}_gep"),
				)?
			}
		};

		Ok(gep)
	}
}
