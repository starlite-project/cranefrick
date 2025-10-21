use inkwell::{
	types::{BasicType, IntType, VectorType},
	values::{BasicValueEnum, IntValue, VectorValue},
};

pub trait LoadableValue<'ctx>: BasicType<'ctx> + super::sealed::Sealed {
	type Value: super::sealed::Sealed + 'ctx;

	fn from_basic_value_enum(value: BasicValueEnum<'ctx>) -> Self::Value;
}

impl<'ctx> LoadableValue<'ctx> for IntType<'ctx> {
	type Value = IntValue<'ctx>;

	fn from_basic_value_enum(value: BasicValueEnum<'ctx>) -> Self::Value {
		value.into_int_value()
	}
}

impl<'ctx> LoadableValue<'ctx> for VectorType<'ctx> {
	type Value = VectorValue<'ctx>;

	fn from_basic_value_enum(value: BasicValueEnum<'ctx>) -> Self::Value {
		value.into_vector_value()
	}
}
