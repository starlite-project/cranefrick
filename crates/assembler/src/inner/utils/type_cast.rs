use inkwell::{
	context::AsContextRef,
	types::{BasicType, IntType},
	values::{BasicValue, BasicValueEnum, IntValue},
};

use crate::ContextGetter as _;

#[repr(transparent)]
pub struct Bool;

// LLVM uses i1 for boolean types
impl<'ctx> Castable<'ctx> for Bool {
	type Type = IntType<'ctx>;
	type Value = IntValue<'ctx>;

	fn cast(v: BasicValueEnum<'ctx>) -> Self::Value {
		v.into_int_value()
	}

	fn assert_type_matches(v: BasicValueEnum<'ctx>, context: impl AsContextRef<'ctx>) {
		let bool_type = context.context().bool_type();

		assert_eq!(v.get_type(), bool_type.into());
	}
}

#[repr(transparent)]
pub struct Int<const N: u32>;

impl<'ctx, const N: u32> Castable<'ctx> for Int<N> {
	type Type = IntType<'ctx>;
	type Value = IntValue<'ctx>;

	fn cast(v: BasicValueEnum<'ctx>) -> Self::Value {
		v.into_int_value()
	}

	fn assert_type_matches(v: BasicValueEnum<'ctx>, context: impl AsContextRef<'ctx>) {
		let matching_ty = context.context().custom_width_int_type(N);

		assert_eq!(v.get_type(), matching_ty.into());
	}
}

pub trait Castable<'ctx> {
	type Type: BasicType<'ctx>;

	type Value: BasicValue<'ctx>;

	fn cast(v: BasicValueEnum<'ctx>) -> Self::Value;

	fn assert_type_matches(v: BasicValueEnum<'ctx>, context: impl AsContextRef<'ctx>);
}
