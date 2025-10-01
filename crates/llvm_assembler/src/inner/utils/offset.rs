use inkwell::values::IntValue;

#[derive(Debug, Clone, Copy)]
pub enum CalculatedOffset<'ctx> {
	Calculated(IntValue<'ctx>),
	Raw(i32),
}

impl From<i32> for CalculatedOffset<'_> {
	fn from(value: i32) -> Self {
		Self::Raw(value)
	}
}

impl<'ctx> From<IntValue<'ctx>> for CalculatedOffset<'ctx> {
	fn from(value: IntValue<'ctx>) -> Self {
		Self::Calculated(value)
	}
}
