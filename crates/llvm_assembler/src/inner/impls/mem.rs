use std::fmt::Display;

use inkwell::{
	types::{BasicType, BasicTypeEnum},
	values::{IntValue, PointerValue},
};

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(
		&self,
		offset: i32,
		fn_name: impl Display,
	) -> Result<IntValue<'ctx>, LlvmAssemblyError> {
		let (loaded_value, ..) = self.load_from(offset, fn_name)?;

		Ok(loaded_value)
	}

	pub fn load_from(
		&self,
		offset: i32,
		fn_name: impl Display,
	) -> Result<(IntValue<'ctx>, PointerValue<'ctx>), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		let current_offset = self.offset_pointer(offset)?;

		let gep = self.gep(i8_type, current_offset, format!("{fn_name}_load_from"))?;

		let loaded_value = self
			.builder
			.build_load(i8_type, gep, &format!("{fn_name}_load_from_load"))?
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
		self.builder.build_store(gep, value)?;

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

		let current_offset = self.offset_pointer(offset)?;

		let gep = self.gep(i8_type, current_offset, fn_name)?;
		self.store_into_inner(value, gep)
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

		match basic_type {
			BasicTypeEnum::IntType(ty) => Ok(unsafe {
				self.builder.build_in_bounds_gep(
					ty,
					self.pointers.tape,
					&[offset],
					&format!("{name}_int_gep"),
				)?
			}),
			BasicTypeEnum::VectorType(ty) => {
				let zero = self.ptr_int_type.const_zero();

				Ok(unsafe {
					self.builder.build_in_bounds_gep(
						ty,
						self.pointers.tape,
						&[zero, offset],
						&format!("{name}_vector_gep"),
					)?
				})
			}
			BasicTypeEnum::ArrayType(ty) => {
				let zero = self.ptr_int_type.const_zero();

				Ok(unsafe {
					self.builder.build_in_bounds_gep(
						ty,
						self.pointers.tape,
						&[zero, offset],
						&format!("{name}_array_gep"),
					)?
				})
			}
			other => Err(LlvmAssemblyError::InvalidGEPType(other.to_string())),
		}
	}
}
