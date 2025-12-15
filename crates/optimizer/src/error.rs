use std::{
	error::Error,
	fmt::{Display, Formatter, Result as FmtResult},
};

use frick_serialize::SerializeError;

use super::InstructionsOptimizerError;

#[derive(Debug)]
pub enum OptimizerError {
	InstructionsOptimizer(InstructionsOptimizerError),
	Serialize(SerializeError),
}

impl Display for OptimizerError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::InstructionsOptimizer(..) => {
				f.write_str("an error occurred optimizing instructions")
			}
			Self::Serialize(..) => f.write_str("an error occurred during serialization"),
		}
	}
}

impl From<InstructionsOptimizerError> for OptimizerError {
	fn from(value: InstructionsOptimizerError) -> Self {
		Self::InstructionsOptimizer(value)
	}
}

impl From<SerializeError> for OptimizerError {
	fn from(value: SerializeError) -> Self {
		Self::Serialize(value)
	}
}

impl Error for OptimizerError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::InstructionsOptimizer(e) => Some(e),
			Self::Serialize(e) => Some(e),
		}
	}
}
