use inkwell::{
	types::{IntType, VectorType},
	values::{IntValue, VectorValue},
};

pub trait Sealed {}

impl Sealed for IntType<'_> {}
impl Sealed for IntValue<'_> {}

impl Sealed for VectorType<'_> {}
impl Sealed for VectorValue<'_> {}
