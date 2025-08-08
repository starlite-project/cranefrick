use std::{
	error::Error as StdError,
	fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Debug, Clone)]
pub enum Error {
	Jit(String),
	Compile(String),
	Optimization(String),
	VariableNotFound(String),
	InvalidExpression(String),
	Numeric(String),
	InvalidInput(String),
	Generic(String),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Jit(msg) => {
				f.write_str("JIT compilation error: ")?;
				f.write_str(msg)
			}
			Self::Compile(msg) => {
				f.write_str("compilation error: ")?;
				f.write_str(msg)
			}
			Self::Optimization(msg) => {
				f.write_str("optimization error: ")?;
				f.write_str(msg)
			}
			Self::VariableNotFound(msg) => {
				f.write_str("variable not found: ")?;
				f.write_str(msg)
			}
			Self::InvalidExpression(msg) => {
				f.write_str("invalid expression: ")?;
				f.write_str(msg)
			}
			Self::Numeric(msg) => {
				f.write_str("numeric error: ")?;
				f.write_str(msg)
			}
			Self::InvalidInput(msg) => {
				f.write_str("invalid input: ")?;
				f.write_str(msg)
			}
			Self::Generic(msg) => {
				f.write_str("error: ")?;
				f.write_str(msg)
			}
		}
	}
}

impl StdError for Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;
