#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "parse")]
mod parse;

use alloc::{string::String, vec::Vec};
use core::ops::Range;

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
	pub const fn ty(&self) -> &BrainOperationType {
		&self.op
	}

	pub const fn ty_mut(&mut self) -> &mut BrainOperationType {
		&mut self.op
	}

	#[must_use]
	pub const fn span(&self) -> Range<usize> {
		self.span.start..self.span.end
	}

	#[must_use]
	pub const fn child_ops(&self) -> Option<&Vec<Self>> {
		match self.ty() {
			BrainOperationType::DynamicLoop(ops) => Some(ops),
			_ => None,
		}
	}

	pub const fn child_ops_mut(&mut self) -> Option<&mut Vec<Self>> {
		match self.ty_mut() {
			BrainOperationType::DynamicLoop(ops) => Some(ops),
			_ => None,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrainOperationType {
	ChangeCell(i8),
	MovePointer(i32),
	InputIntoCell,
	OutputCurrentCell,
	DynamicLoop(Vec<BrainOperation>),
	Comment(String),
}
