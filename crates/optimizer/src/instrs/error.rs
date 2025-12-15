use std::{
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
				f.write_str(register_type(Some(expected)))?;
				f.write_str(", found ")?;
				f.write_str(register_type(found))
			}
		}
	}
}

impl Error for InstructionsOptimizerError {}

const fn register_type(reg: Option<RegisterTypeEnum>) -> &'static str {
	match reg {
		Some(RegisterTypeEnum::Any) => "a value",
		Some(RegisterTypeEnum::Bool) => "a boolean",
		Some(RegisterTypeEnum::Int) => "an integer",
		Some(RegisterTypeEnum::Pointer) => "a pointer",
		None => "nothing",
	}
}
