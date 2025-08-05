use std::{
	error::Error as StdError,
	fmt::{Display, Formatter, Result as FmtResult},
	io,
};

#[derive(Debug)]
#[repr(transparent)]
pub struct Error {
	inner: Box<InnerError>,
}

impl Error {
	pub fn message(message: impl Into<String>) -> Self {
		Self {
			inner: Box::new(InnerError::Message(message.into())),
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		Display::fmt(&self.inner, f)
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match &*self.inner {
			InnerError::Io(e) => Some(e),
			InnerError::Message(..) => None,
		}
	}
}

impl From<io::Error> for Error {
	fn from(value: io::Error) -> Self {
		Self {
			inner: Box::new(InnerError::Io(value)),
		}
	}
}

#[derive(Debug)]
enum InnerError {
	Message(String),
	Io(io::Error),
}

impl Display for InnerError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Message(s) => f.write_str(s),
			Self::Io(..) => f.write_str("an io error occurred"),
		}
	}
}
