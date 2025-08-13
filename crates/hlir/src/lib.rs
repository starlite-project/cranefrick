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
use tracing::{Span, debug, info, trace};
use tracing_indicatif::{span_ext::IndicatifSpanExt as _, style::ProgressStyle};

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

	#[tracing::instrument(skip(self))]
	pub fn parse<I>(self) -> Result<I, ParseError>
	where
		I: Default + Extend<BrainHlir>,
	{
		let len = self.inner.source().len();

		{
			let span = Span::current();

			span.pb_set_style(&ProgressStyle::with_template("{span_child_prefix}{spinner} {span_name}({span_fields}) [{elapsed_precise}] [{bar:38}] ({eta})").unwrap().progress_chars("#>-"));
			span.pb_set_length(len as u64);
		}

		info!(len = len, "scanning chars");

		let mut result = I::default();

		let mut bracket_stack = Vec::new();

		for (i, op) in self.inner.filter_map(Result::ok).enumerate() {
			trace!(op = %op);

			let repr = match op {
				InnerOpCode::Clear => BrainHlir::ClearCell,
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

			Span::current().pb_inc(1);

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
	/// Clear current cell ([-]).
	ClearCell,
}

impl Display for BrainHlir {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match *self {
			Self::MovePtrLeft => f.write_char('<'),
			Self::MovePtrRight => f.write_char('>'),
			Self::IncrementCell => f.write_char('+'),
			Self::DecrementCell => f.write_char('-'),
			Self::GetInput => f.write_char(','),
			Self::PutOutput => f.write_char('.'),
			Self::StartLoop => f.write_char('['),
			Self::EndLoop => f.write_char(']'),
			Self::ClearCell => f.write_str("[-]"),
		}
	}
}

#[derive(Debug, Clone, Copy)]
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
