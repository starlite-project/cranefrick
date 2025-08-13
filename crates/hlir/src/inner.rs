use core::fmt::{Display, Formatter, Result as FmtResult, Write as _};

use logos::Logos;

/// This is here so we don't leak the [`Logos`] implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Logos)]
pub enum InnerOpCode {
	#[token("<")]
	MoveLeft,
	#[token(">")]
	MoveRight,
	#[token("+")]
	Increment,
	#[token("-")]
	Decrement,
	#[token(",")]
	Input,
	#[token(".")]
	Output,
	#[token("[")]
	StartLoop,
	#[token("]")]
	EndLoop,
	#[token("[-]")]
	Clear,
}

impl Display for InnerOpCode {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match *self {
			Self::MoveLeft => f.write_char('<'),
			Self::MoveRight => f.write_char('>'),
			Self::Increment => f.write_char('+'),
			Self::Decrement => f.write_char('-'),
			Self::Input => f.write_char(','),
			Self::Output => f.write_char('.'),
			Self::StartLoop => f.write_char('['),
			Self::EndLoop => f.write_char(']'),
			Self::Clear => f.write_str("[-]"),
		}
	}
}
