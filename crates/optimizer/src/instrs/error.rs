use std::{
	error::Error,
	fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Debug, Clone)]
pub enum InstructionsOptimizerError {
	InstructionsNotValid,
}

impl Display for InstructionsOptimizerError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match *self {
			Self::InstructionsNotValid => {
				f.write_str("instructions not valid, a pass has malformed them.")
			}
		}
	}
}

impl Error for InstructionsOptimizerError {}
