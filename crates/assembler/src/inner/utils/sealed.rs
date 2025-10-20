use inkwell::{types::IntType, values::IntValue};

pub trait Sealed {}

impl Sealed for IntType<'_> {}
impl Sealed for IntValue<'_> {}
