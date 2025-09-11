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

		self.load_into(offset, format!("{fn_name}"))?;

		let loaded_value = self
			.builder
			.build_load(i8_type, self.load, &format!("{fn_name}_load_load"))?
			.into_int_value();

		Ok(loaded_value)
	}

	pub fn load_from(
		&self,
		offset: i32,
		fn_name: impl Display,
	) -> Result<(IntValue<'ctx>, PointerValue<'ctx>), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		let i8_size = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(1, false)
		};

		let current_offset = self.offset_ptr(offset)?;

		let gep = self.gep(i8_type, current_offset, format!("{fn_name}_load_from"))?;

		self.builder.build_memcpy(self.load, 1, gep, 1, i8_size)?;

		let loaded_value = self
			.builder
			.build_load(i8_type, self.load, &format!("{fn_name}_load_from_load"))?
			.into_int_value();

		Ok((loaded_value, gep))
	}

	pub fn store_value_into(
		&self,
		value: u8,
		gep: PointerValue<'ctx>,
	) -> Result<(), LlvmAssemblyError> {
		let value = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(value.into(), false)
		};

		self.store_into_inner(value, gep)
	}

	pub fn store_into(
		&self,
		value: IntValue<'ctx>,
		gep: PointerValue<'ctx>,
	) -> Result<(), LlvmAssemblyError> {
		self.store_into_inner(value, gep)
	}

	fn store_into_inner(
		&self,
		value: IntValue<'ctx>,
		gep: PointerValue<'ctx>,
	) -> Result<(), LlvmAssemblyError> {
		let i8_size = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(1, false)
		};

		self.builder.build_call(
			self.functions.lifetime.start,
			&[i8_size.into(), self.store.into()],
			"",
		)?;

		self.builder.build_store(self.store, value)?;

		self.builder.build_memcpy(gep, 1, self.store, 1, i8_size)?;

		self.builder.build_call(
			self.functions.lifetime.end,
			&[i8_size.into(), self.store.into()],
			"",
		)?;

		Ok(())
	}

	fn load_into(&self, offset: i32, fn_name: String) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();
		let i8_size = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(1, false)
		};

		let current_offset = self.offset_ptr(offset)?;

		let value = self.gep(i8_type, current_offset, format!("{fn_name}_load_into"))?;

		self.builder.build_memcpy(self.load, 1, value, 1, i8_size)?;

		Ok(())
	}

	pub fn take(
		&self,
		offset: i32,
		fn_name: impl Display,
	) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let (value, gep) = self.load_from(offset, fn_name)?;

		self.store_value_into(0, gep)?;

		Ok(value)
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

		self.builder.build_call(
			self.functions.lifetime.start,
			&[i8_size.into(), self.store.into()],
			"",
		)?;

		self.builder.build_store(self.store, value)?;

		let current_offset = self.offset_ptr(offset)?;

		let current_tape_value = self.gep(i8_type, current_offset, fn_name)?;

		self.builder.build_memcpy(
			current_tape_value,
			1,
			self.store,
			1,
			self.context().i64_type().const_int(1, false),
		)?;

		self.builder.build_call(
			self.functions.lifetime.end,
			&[i8_size.into(), self.store.into()],
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
