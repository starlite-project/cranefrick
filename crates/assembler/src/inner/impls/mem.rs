use std::{cell::RefCell, sync::Once};

use frick_utils::Convert as _;
use inkwell::{
	builder::Builder,
	types::{BasicType, BasicTypeEnum, IntType},
	values::{BasicValue, IntValue, PointerValue},
};

use super::create_string;
use crate::{
	AssemblyError, ContextGetter as _,
	inner::{
		InnerAssembler,
		utils::{CalculatedOffset, LoadableValue},
	},
};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip(self))]
	pub fn load_cell(&self, offset: i32, fn_name: &str) -> Result<IntValue<'ctx>, AssemblyError> {
		let (loaded_value, ..) = self.load_cell_and_pointer(offset, fn_name)?;

		Ok(loaded_value)
	}

	pub fn load_from<T: LoadableValue<'ctx>>(
		&self,
		value_ty: T,
		gep: PointerValue<'ctx>,
		fn_name: &str,
	) -> Result<T::Value, AssemblyError> {
		let loaded_value =
			self.builder
				.build_load(value_ty, gep, &create_string(fn_name, "_load_from_load\0"))?;

		Ok(T::from_basic_value_enum(loaded_value))
	}

	#[tracing::instrument(skip(self))]
	pub fn load_cell_and_pointer(
		&self,
		offset: i32,
		fn_name: &str,
	) -> Result<(IntValue<'ctx>, PointerValue<'ctx>), AssemblyError> {
		let i8_type = self.context().i8_type();

		let gep = self.tape_gep(
			i8_type,
			offset,
			&create_string(fn_name, "_load_cell_and_pointer"),
		)?;

		let loaded_value = self.load_from(
			i8_type,
			gep,
			&create_string(fn_name, "_load_cell_and_pointer\0"),
		)?;

		Ok((loaded_value, gep))
	}

	#[tracing::instrument(skip(self))]
	pub fn store_value_into(
		&self,
		value: u8,
		gep: PointerValue<'ctx>,
	) -> Result<(), AssemblyError> {
		let value = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(value.convert::<u64>(), false)
		};

		self.store_into(value, gep)
	}

	#[tracing::instrument(skip(self))]
	pub fn store_into(
		&self,
		value: impl BasicValue<'ctx>,
		gep: PointerValue<'ctx>,
	) -> Result<(), AssemblyError> {
		self.builder.build_store(gep, value)?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub fn take(&self, offset: i32, fn_name: &str) -> Result<IntValue<'ctx>, AssemblyError> {
		let (value, gep) = self.load_cell_and_pointer(offset, fn_name)?;

		self.store_value_into(0, gep)?;

		Ok(value)
	}

	#[tracing::instrument(skip(self))]
	pub fn store_value_into_cell(
		&self,
		value: u8,
		offset: i32,
		fn_name: &str,
	) -> Result<(), AssemblyError> {
		let value = {
			let i8_type = self.context().i8_type();

			i8_type.const_int(value.convert::<u64>(), false)
		};

		self.store_into_cell_inner(value, offset, create_string(fn_name, "_store_value\0"))
	}

	#[tracing::instrument(skip(self))]
	pub fn store_into_cell(
		&self,
		value: impl BasicValue<'ctx>,
		offset: i32,
		fn_name: &str,
	) -> Result<(), AssemblyError> {
		self.store_into_cell_inner(value, offset, create_string(fn_name, "_store\0"))
	}

	#[tracing::instrument(skip(self))]
	fn store_into_cell_inner(
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

	#[tracing::instrument(skip(self), fields(offset = ?CalculatedOffset::from(offset)))]
	#[inline]
	pub fn tape_gep<O>(
		&self,
		ty: impl BasicType<'ctx>,
		offset: O,
		name: &str,
	) -> Result<PointerValue<'ctx>, AssemblyError>
	where
		CalculatedOffset<'ctx>: From<O>,
		O: Copy,
	{
		self.gep(ty, self.pointers.tape, offset, &format!("{name}_tape"))
	}

	#[tracing::instrument(skip(self), fields(offset = ?CalculatedOffset::from(offset)))]
	pub fn gep<O>(
		&self,
		ty: impl BasicType<'ctx>,
		ptr: PointerValue<'ctx>,
		offset: O,
		name: &str,
	) -> Result<PointerValue<'ctx>, AssemblyError>
	where
		CalculatedOffset<'ctx>: From<O>,
		O: Copy,
	{
		let offset = self.resolve_offset(
			offset.convert::<CalculatedOffset<'ctx>>(),
			format!("{name}_gep"),
		)?;

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
						&create_string(name, "_array_gep\0"),
					)?
				})
			}
			BasicTypeEnum::IntType(ty) => Ok(unsafe {
				self.builder.build_in_bounds_gep(
					ty,
					ptr,
					&[offset],
					&create_string(name, "_int_gep\0"),
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
						&create_string(name, "_vector_gep\0"),
					)?
				})
			}
			other => Err(AssemblyError::InvalidGEPType(other.to_string())),
		}
	}
}

pub struct Slot<'builder, 'ctx> {
	builder: &'builder Builder<'ctx>,
	ptr: PointerValue<'ctx>,
	was_set: RefCell<bool>,
	i8_type: IntType<'ctx>,
}

impl<'ctx> Slot<'_, 'ctx> {
	pub fn set(&self, value: impl BasicValue<'ctx>) -> Result<(), AssemblyError> {
		if *self.was_set.borrow() {
			return Err(AssemblyError::SlotAlreadySet);
		}

		self.builder.build_store(self.ptr, value)?;

		*self.was_set.borrow_mut() = true;

		Ok(())
	}
}

impl Drop for Slot<'_, '_> {
	fn drop(&mut self) {
		if !*self.was_set.borrow() {
			self.builder
				.build_store(self.ptr, self.i8_type.get_poison())
				.unwrap();
		}
	}
}
