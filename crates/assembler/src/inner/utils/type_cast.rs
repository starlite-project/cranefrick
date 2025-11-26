use frick_types::{Bool, Int, Pointer, RegisterType};
use inkwell::{
	types::{BasicType, IntType, PointerType},
	values::{BasicValue, IntValue, PointerValue},
};

pub trait Castable<'ctx>: RegisterType {
	type Type: BasicType<'ctx> + Copy;

	type Value: BasicValue<'ctx> + Copy;

	fn cast(v: impl BasicValue<'ctx>) -> Self::Value;

	fn assert_type_matches(v: impl BasicValue<'ctx>);
}

// LLVM uses i1 for boolean types
impl<'ctx> Castable<'ctx> for Bool {
	type Type = IntType<'ctx>;
	type Value = IntValue<'ctx>;

	fn cast(v: impl BasicValue<'ctx>) -> Self::Value {
		v.as_basic_value_enum().into_int_value()
	}

	fn assert_type_matches(v: impl BasicValue<'ctx>) {
		assert!(v.as_basic_value_enum().is_int_value());
	}
}

impl<'ctx> Castable<'ctx> for Int {
	type Type = IntType<'ctx>;
	type Value = IntValue<'ctx>;

	fn cast(v: impl BasicValue<'ctx>) -> Self::Value {
		v.as_basic_value_enum().into_int_value()
	}

	fn assert_type_matches(v: impl BasicValue<'ctx>) {
		assert!(v.as_basic_value_enum().is_int_value());
	}
}

impl<'ctx> Castable<'ctx> for Pointer {
	type Type = PointerType<'ctx>;
	type Value = PointerValue<'ctx>;

	fn cast(v: impl BasicValue<'ctx>) -> Self::Value {
		v.as_any_value_enum().into_pointer_value()
	}

	fn assert_type_matches(v: impl BasicValue<'ctx>) {
		assert!(v.as_basic_value_enum().is_pointer_value());
	}
}
