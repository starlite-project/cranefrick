use std::{fmt::Display, ops::RangeInclusive};

use inkwell::{
	types::BasicType,
	values::{IntValue, PointerValue},
};

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(
		&self,
		offset: i32,
		fn_name: impl Display,
	) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		let current_offset = self.offset_ptr(offset)?;

		let value = self.gep(i8_type, current_offset, format!("{fn_name}_load"))?;

		let new_value_slot = self.builder.build_alloca(i8_type, "load_alloca")?;

		self.builder
			.build_memcpy(new_value_slot, 1, value, 1, i8_type.const_int(1, false))?;

		let loaded_value = self
			.builder
			.build_load(i8_type, new_value_slot, &format!("{fn_name}_load_load"))?
			.into_int_value();

		if let Some(instr) = loaded_value.as_instruction() {
			let noundef_metadata_id = self.context().get_kind_id("noundef");
			let noalias_metadata_id = self.context().get_kind_id("noalias");
			let empty_metadata_node = self.context().metadata_node(&[]);

			instr
				.set_metadata(empty_metadata_node, noundef_metadata_id)
				.map_err(|_| LlvmAssemblyError::InvalidMetadata)?;

			instr
				.set_metadata(empty_metadata_node, noalias_metadata_id)
				.map_err(|_| LlvmAssemblyError::InvalidMetadata)?;
		}

		Ok(loaded_value)
	}

	pub fn load_into(
		&self,
		slot: PointerValue<'ctx>,
		offset: i32,
	) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();
		let i8_size = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(1, false)
		};

		let current_offset = self.offset_ptr(offset)?;

		let value = self.gep(i8_type, current_offset, "load_into")?;

		self.builder.build_memcpy(slot, 1, value, 1, i8_size)?;

		Ok(())
	}

	pub fn store_value(
		&self,
		value: u8,
		offset: i32,
		fn_name: impl Display,
	) -> Result<(), LlvmAssemblyError> {
		let value = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(value.into(), false)
		};

		self.store_inner(value, offset, format!("{fn_name}_store_value"))
	}

	pub fn store(
		&self,
		value: IntValue<'ctx>,
		offset: i32,
		fn_name: impl Display,
	) -> Result<(), LlvmAssemblyError> {
		self.store_inner(value, offset, format!("{fn_name}_store"))
	}

	fn store_inner(
		&self,
		value: IntValue<'ctx>,
		offset: i32,
		fn_name: String,
	) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		let i8_size = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(1, false)
		};

		let new_alloca_slot = self
			.builder
			.build_alloca(i8_type, &format!("{fn_name}_alloca"))?;

		self.builder.build_call(
			self.functions.lifetime.start,
			&[i8_size.into(), new_alloca_slot.into()],
			"",
		)?;

		self.builder.build_store(new_alloca_slot, value)?;

		let current_offset = self.offset_ptr(offset)?;

		let current_tape_value = self.gep(i8_type, current_offset, fn_name)?;

		self.builder.build_memcpy(
			current_tape_value,
			1,
			new_alloca_slot,
			1,
			self.context().i64_type().const_int(1, false),
		)?;

		self.builder.build_call(
			self.functions.lifetime.end,
			&[i8_size.into(), new_alloca_slot.into()],
			"",
		)?;

		Ok(())
	}

	pub fn mem_set(&self, value: u8, range: RangeInclusive<i32>) -> Result<(), LlvmAssemblyError> {
		let start = *range.start();
		let range_len = range.count();
		let i8_type = self.context().i8_type();

		let range_len_value = {
			let ptr_int_type = self.ptr_int_type;

			ptr_int_type.const_int(range_len as u64, false)
		};

		let array_alloca =
			self.builder
				.build_array_alloca(i8_type, range_len_value, "mem_set_alloca")?;

		self.builder.build_call(
			self.functions.lifetime.start,
			&[range_len_value.into(), array_alloca.into()],
			"",
		)?;

		let value_value = i8_type.const_int(value.into(), false);

		self.builder
			.build_memset(array_alloca, 1, value_value, range_len_value)?;

		let current_offset = self.offset_ptr(start)?;

		let gep = self.gep(i8_type, current_offset, "mem_set")?;

		self.builder
			.build_memcpy(gep, 1, array_alloca, 1, range_len_value)?;

		self.builder.build_call(
			self.functions.lifetime.end,
			&[range_len_value.into(), array_alloca.into()],
			"",
		)?;

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
