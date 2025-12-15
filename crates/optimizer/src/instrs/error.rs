use std::{
	borrow::Cow,
	error::Error,
	fmt::{Display, Formatter, Result as FmtResult},
};

use frick_types::RegisterTypeEnum;

#[derive(Debug, Clone)]
pub enum InstructionsOptimizerError {
	LoopsNotValid,
	RegisterInvalid {
		register: usize,
		expected: RegisterTypeEnum,
		found: Option<RegisterTypeEnum>,
	},
}

impl Display for InstructionsOptimizerError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match *self {
			Self::LoopsNotValid => f.write_str("loop start/end count is not equal"),
			Self::RegisterInvalid {
				register,
				expected,
				found,
			} => {
				f.write_str("register(")?;
				Display::fmt(&register, f)?;
				f.write_str(") is invalid; expected ")?;
				f.write_str(&*register_type(Some(expected)))?;
				f.write_str(", found ")?;
				f.write_str(&*register_type(found))
			}
		}
	}
}

impl Error for InstructionsOptimizerError {}

fn register_type(reg: Option<RegisterTypeEnum>) -> Cow<'static, str> {
	match reg {
		Some(RegisterTypeEnum::Any) => Cow::Borrowed("a value"),
		Some(RegisterTypeEnum::Bool) => Cow::Borrowed("a boolean"),
		Some(RegisterTypeEnum::Int(Some(size))) => Cow::Owned(format!("an int{size}")),
		Some(RegisterTypeEnum::Int(None)) => Cow::Borrowed("an int"),
		Some(RegisterTypeEnum::Pointer) => Cow::Borrowed("a pointer"),
		None => Cow::Borrowed("nothing"),
	}
}
