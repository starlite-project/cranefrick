#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod cell_offset_options;
#[cfg(feature = "parse")]
mod parse;

use alloc::vec::Vec;
use core::ops::{Deref, DerefMut, Range};

use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

pub use self::cell_offset_options::*;
#[cfg(feature = "parse")]
pub use self::parse::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BrainOperation {
	op: BrainOperationType,
	#[serde(skip)]
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

	pub const fn op_mut(&mut self) -> &mut BrainOperationType {
		&mut self.op
	}

	#[must_use]
	pub const fn span(&self) -> Range<usize> {
		self.span.start..self.span.end
	}
}

impl Deref for BrainOperation {
	type Target = BrainOperationType;

	fn deref(&self) -> &Self::Target {
		&self.op
	}
}

impl DerefMut for BrainOperation {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.op
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainOperationType {
	IncrementCell(CellOffsetOptions),
	DecrementCell(CellOffsetOptions),
	SetCell(CellOffsetOptions),
	MovePointer(i32),
	MoveCellValue(CellOffsetOptions),
	TakeCellValue(CellOffsetOptions),
	InputIntoCell,
	OutputCell(CellOffsetOptions),
	OutputValue(u8),
	DynamicLoop(Vec<BrainOperation>),
	Comment(char),
}

impl BrainOperationType {
	#[must_use]
	pub const fn is_zeroing_cell(&self) -> bool {
		matches!(
			self,
			Self::DynamicLoop(..)
				| Self::SetCell(CellOffsetOptions {
					value: 0,
					offset: 0
				}) | Self::MoveCellValue(..)
		)
	}

	#[must_use]
	pub fn has_io(&self) -> bool {
		match self {
			Self::InputIntoCell | Self::OutputCell(..) | Self::OutputValue(..) => true,
			Self::DynamicLoop(ops) => ops.iter().any(|i| i.has_io()),
			_ => false,
		}
	}

	#[must_use]
	pub const fn child_ops(&self) -> Option<&Vec<BrainOperation>> {
		match self {
			Self::DynamicLoop(ops) => Some(ops),
			_ => None,
		}
	}

	pub const fn child_ops_mut(&mut self) -> Option<&mut Vec<BrainOperation>> {
		match self {
			Self::DynamicLoop(ops) => Some(ops),
			_ => None,
		}
	}

	#[must_use]
	pub const fn increment_cell(value: u8) -> Self {
		Self::increment_cell_at(value, 0)
	}

	#[must_use]
	pub const fn increment_cell_at(value: u8, offset: i32) -> Self {
		Self::IncrementCell(CellOffsetOptions::new(value, offset))
	}

	#[must_use]
	pub const fn decrement_cell(value: u8) -> Self {
		Self::decrement_cell_at(value, 0)
	}

	#[must_use]
	pub const fn decrement_cell_at(value: u8, offset: i32) -> Self {
		Self::DecrementCell(CellOffsetOptions::new(value, offset))
	}

	#[must_use]
	pub const fn set_cell(value: u8) -> Self {
		Self::set_cell_at(value, 0)
	}

	#[must_use]
	pub const fn set_cell_at(value: u8, offset: i32) -> Self {
		Self::SetCell(CellOffsetOptions::new(value, offset))
	}

	#[must_use]
	pub const fn clear_cell() -> Self {
		Self::clear_cell_at(0)
	}

	#[must_use]
	pub const fn clear_cell_at(offset: i32) -> Self {
		Self::set_cell_at(0, offset)
	}

	#[must_use]
	pub const fn offset(&self) -> Option<i32> {
		match self {
			Self::IncrementCell(CellOffsetOptions { offset, .. })
			| Self::DecrementCell(CellOffsetOptions { offset, .. })
			| Self::SetCell(CellOffsetOptions { offset, .. })
			| Self::OutputCell(CellOffsetOptions { offset, .. }) => Some(*offset),
			_ => None,
		}
	}
}
