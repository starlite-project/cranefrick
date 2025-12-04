use frick_types::{Any, Bool, Int, Pointer, RegisterType};
use inkwell::{
	types::{BasicType, BasicTypeEnum, IntType, PointerType},
	values::{BasicValue, BasicValueEnum, IntValue, PointerValue},
};

pub trait Castable<'ctx>: RegisterType {
	type Type: BasicType<'ctx> + Copy;

	type Value: BasicValue<'ctx> + Copy;

	fn cast(v: impl BasicValue<'ctx>) -> Self::Value;

	fn assert_type_matches(v: impl BasicValue<'ctx>);
}

impl<'ctx> Castable<'ctx> for Any {
	type Type = BasicTypeEnum<'ctx>;
	type Value = BasicValueEnum<'ctx>;

	fn cast(v: impl BasicValue<'ctx>) -> Self::Value {
		v.as_basic_value_enum()
	}

	fn assert_type_matches(_: impl BasicValue<'ctx>) {}
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
