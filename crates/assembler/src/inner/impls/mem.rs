use inkwell::{
	types::{BasicType, BasicTypeEnum},
	values::{BasicValue, IntValue, PointerValue},
};

use super::create_string;
use crate::{
	AssemblyError, ContextGetter as _,
	inner::{InnerAssembler, utils::CalculatedOffset},
};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn load(&self, offset: i32, fn_name: &str) -> Result<IntValue<'ctx>, AssemblyError> {
		let (loaded_value, ..) = self.load_from(offset, fn_name)?;

		Ok(loaded_value)
	}

	pub fn load_from(
		&self,
		offset: i32,
		fn_name: &str,
	) -> Result<(IntValue<'ctx>, PointerValue<'ctx>), AssemblyError> {
		let i8_type = self.context().i8_type();

		let gep = self.tape_gep(i8_type, offset, &create_string(fn_name, "_load_from"))?;

		let loaded_value = self
			.builder
			.build_load(i8_type, gep, &create_string(fn_name, "_load_from_load"))?
			.into_int_value();

		Ok((loaded_value, gep))
	}

	pub fn store_value_into(
		&self,
		value: u8,
		gep: PointerValue<'ctx>,
	) -> Result<(), AssemblyError> {
		let value = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(value.into(), false)
		};

		self.store_into(value, gep)
	}

	pub fn store_into(
		&self,
		value: impl BasicValue<'ctx>,
		gep: PointerValue<'ctx>,
	) -> Result<(), AssemblyError> {
		self.builder.build_store(gep, value)?;

		Ok(())
	}

	pub fn take(&self, offset: i32, fn_name: &str) -> Result<IntValue<'ctx>, AssemblyError> {
		let (value, gep) = self.load_from(offset, fn_name)?;

		self.store_value_into(0, gep)?;

		Ok(value)
	}

	pub fn store_value(&self, value: u8, offset: i32, fn_name: &str) -> Result<(), AssemblyError> {
		let value = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(value.into(), false)
		};

		self.store_inner(value, offset, create_string(fn_name, "_store_value"))
	}

	pub fn store(
		&self,
		value: impl BasicValue<'ctx>,
		offset: i32,
		fn_name: &str,
	) -> Result<(), AssemblyError> {
		self.store_inner(value, offset, create_string(fn_name, "_store"))
	}

	fn store_inner(
		&self,
		value: impl BasicValue<'ctx>,
		offset: i32,
		fn_name: String,
	) -> Result<(), AssemblyError> {
		let i8_type = self.context().i8_type();

		let gep = self.tape_gep(i8_type, offset, &fn_name)?;
		self.store_into(value, gep)?;

		Ok(())
	}

	#[inline]
	pub fn tape_gep(
		&self,
		ty: impl BasicType<'ctx>,
		offset: impl Into<CalculatedOffset<'ctx>>,
		name: &str,
	) -> Result<PointerValue<'ctx>, AssemblyError> {
		self.gep(ty, self.pointers.tape, offset, name)
	}

	pub fn gep(
		&self,
		ty: impl BasicType<'ctx>,
		ptr: PointerValue<'ctx>,
		offset: impl Into<CalculatedOffset<'ctx>>,
		name: &str,
	) -> Result<PointerValue<'ctx>, AssemblyError> {
		let offset = self.resolve_offset(offset.into())?;

		let basic_type = ty.as_basic_type_enum();

		match basic_type {
			BasicTypeEnum::ArrayType(ty) => {
				let zero = {
					let i64_type = self.context().i64_type();

					i64_type.const_zero()
				};

				Ok(unsafe {
					self.builder.build_in_bounds_gep(
						ty,
						ptr,
						&[zero, offset],
						&create_string(name, "_array_gep"),
					)?
				})
			}
			BasicTypeEnum::IntType(ty) => Ok(unsafe {
				self.builder.build_in_bounds_gep(
					ty,
					ptr,
					&[offset],
					&create_string(name, "_int_gep"),
				)?
			}),
			BasicTypeEnum::VectorType(ty) => {
				let zero = {
					let i64_type = self.context().i64_type();

					i64_type.const_zero()
				};

				Ok(unsafe {
					self.builder.build_in_bounds_gep(
						ty,
						ptr,
						&[zero, offset],
						&create_string(name, "_vector_gep"),
					)?
				})
			}
			other => Err(AssemblyError::InvalidGEPType(other.to_string())),
		}
	}
}
