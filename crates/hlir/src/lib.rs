#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

extern crate alloc;

mod inner;

use alloc::vec::Vec;
use core::{
	error::Error as CoreError,
	fmt::{Display, Formatter, Result as FmtResult, Write as _},
	iter::once,
};

use logos::Lexer;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use self::inner::InnerOpCode;

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Parser<'source> {
	inner: Lexer<'source, InnerOpCode>,
}

impl<'source> Parser<'source> {
	pub fn new(source: &'source str) -> Self {
		debug!("got source with length {}", source.len());

		Self {
			inner: Lexer::new(source),
		}
	}

	pub fn parse<I>(self) -> Result<I, ParseError>
	where
		I: Default + Extend<BrainHlir>,
	{
		info!("scanning {} chars", self.inner.source().len());

		let mut result = I::default();

		let mut bracket_stack = Vec::new();

		for (i, op) in self.inner.filter_map(Result::ok).enumerate() {
			let repr = match op {
				InnerOpCode::MoveLeft => BrainHlir::MovePtrLeft,
				InnerOpCode::MoveRight => BrainHlir::MovePtrRight,
				InnerOpCode::Increment => BrainHlir::IncrementCell,
				InnerOpCode::Decrement => BrainHlir::DecrementCell,
				InnerOpCode::Input => BrainHlir::GetInput,
				InnerOpCode::Output => BrainHlir::PutOutput,
				InnerOpCode::StartLoop => {
					bracket_stack.push(i);
					BrainHlir::StartLoop
				}
				InnerOpCode::EndLoop => match bracket_stack.pop() {
					Some(_) => BrainHlir::EndLoop,
					None => return Err(ParseError(i)),
				},
			};

			result.extend(once(repr));
		}

		Ok(result)
	}
}

/// High-level intermediate representation. 1 to 1 for it's brainfuck equivalent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrainHlir {
	/// Move the pointer left (<).
	MovePtrLeft,
	/// Move the pointer right (>).
	MovePtrRight,
	/// Increment the current cell (+).
	IncrementCell,
	/// Decrement the current cell (-).
	DecrementCell,
	/// Get the input from the input stream (,).
	GetInput,
	/// Put the current cell into the output stream (.).
	PutOutput,
	/// Start of a loop ([).
	StartLoop,
	/// End of a loop (]).
	EndLoop,
}

impl Display for BrainHlir {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_char(match *self {
			Self::MovePtrLeft => '<',
			Self::MovePtrRight => '>',
			Self::IncrementCell => '+',
			Self::DecrementCell => '-',
			Self::GetInput => ',',
			Self::PutOutput => '.',
			Self::StartLoop => '[',
			Self::EndLoop => ']',
		})
	}
}

#[derive(Debug)]
pub struct ParseError(usize);

impl Display for ParseError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str("loop ending at #")?;
		Display::fmt(&self.0, f)?;
		f.write_str(" has no beginning")
	}
}

impl CoreError for ParseError {}

#[cfg(test)]
mod tests {
	use alloc::vec::Vec;

	use super::{BrainHlir, ParseError, Parser};

	#[test]
	fn basic_inc() -> Result<(), ParseError> {
		let parsed = Parser::new("+++++").parse::<Vec<_>>()?;

		assert_eq!(parsed, [BrainHlir::IncrementCell; 5]);

		Ok(())
	}
}
