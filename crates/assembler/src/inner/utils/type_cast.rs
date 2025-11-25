use inkwell::{
	context::AsContextRef,
	types::{BasicType, IntType},
	values::{BasicValue, IntValue},
};

use crate::ContextGetter as _;

#[repr(transparent)]
pub struct Bool;

// LLVM uses i1 for boolean types
impl<'ctx> Castable<'ctx> for Bool {
	type Type = IntType<'ctx>;
	type Value = IntValue<'ctx>;

	fn cast(v: impl BasicValue<'ctx>) -> Self::Value {
		v.as_basic_value_enum().into_int_value()
	}

	fn assert_type_matches(v: impl BasicValue<'ctx>, context: impl AsContextRef<'ctx>) {
		let bool_type = context.context().bool_type();

		assert_eq!(v.as_basic_value_enum().get_type(), bool_type.into());
	}
}

#[repr(transparent)]
pub struct Int<const N: u32>;

impl<'ctx, const N: u32> Castable<'ctx> for Int<N> {
	type Type = IntType<'ctx>;
	type Value = IntValue<'ctx>;

	fn cast(v: impl BasicValue<'ctx>) -> Self::Value {
		v.as_basic_value_enum().into_int_value()
	}

	fn assert_type_matches(v: impl BasicValue<'ctx>, context: impl AsContextRef<'ctx>) {
		let matching_ty = context.context().custom_width_int_type(N);

		assert_eq!(v.as_basic_value_enum().get_type(), matching_ty.into());
	}
}

pub trait Castable<'ctx> {
	type Type: BasicType<'ctx>;

	type Value: BasicValue<'ctx>;

	fn cast(v: impl BasicValue<'ctx>) -> Self::Value;

	fn assert_type_matches(v: impl BasicValue<'ctx>, context: impl AsContextRef<'ctx>);
}
