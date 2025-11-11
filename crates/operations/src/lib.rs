#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "parse")]
mod parse;

use alloc::vec::Vec;
use core::{
	fmt::{Debug, Display, Formatter, Result as FmtResult, Write as _},
	ops::Range,
};

use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

pub use self::parse::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainOperation {
	op: BrainOperationType,
	span: Range<usize>,
}

impl BrainOperation {
	#[must_use]
	pub const fn new(op: BrainOperationType, span: Range<usize>) -> Self {
		Self { op, span }
	}

	#[must_use]
	pub const fn move_pointer(offset: i32, span: Range<usize>) -> Self {
		Self::new(BrainOperationType::MovePointer(offset), span)
	}

	#[must_use]
	pub fn dynamic_loop(ops: impl IntoIterator<Item = Self>, span: Range<usize>) -> Self {
		Self::new(BrainOperationType::DynamicLoop(ops.collect_to()), span)
	}

	#[must_use]
	pub const fn op(&self) -> &BrainOperationType {
		&self.op
	}

	#[must_use]
	pub const fn span(&self) -> Range<usize> {
		self.span.start..self.span.end
	}
}

impl Display for BrainOperation {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		Display::fmt(&self.op(), f)
	}
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrainOperationType {
	ChangeCell(i8),
	MovePointer(i32),
	InputIntoCell,
	OutputCurrentCell,
	DynamicLoop(Vec<BrainOperation>),
}

impl Debug for BrainOperationType {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		Display::fmt(&self, f)
	}
}

impl Display for BrainOperationType {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::ChangeCell(value_offset) => {
				let char = if value_offset.is_negative() { '-' } else { '+' };
				for _ in 0..value_offset.unsigned_abs() {
					f.write_char(char)?;
				}
			}
			Self::MovePointer(offset) => {
				let char = if offset.is_negative() { '<' } else { '>' };
				for _ in 0..offset.unsigned_abs() {
					f.write_char(char)?;
				}
			}
			Self::InputIntoCell => f.write_char(',')?,
			Self::OutputCurrentCell => f.write_char('.')?,
			Self::DynamicLoop(ops) => {
				f.write_char('[')?;
				for op in ops {
					Display::fmt(&op.op(), f)?;
				}
				f.write_char(']')?;
			}
		}

		Ok(())
	}
}
