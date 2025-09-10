#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod compiler;

use std::{num::NonZero, ops::RangeInclusive};

use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

pub use self::compiler::*;

/// Mid-level intermediate representation. Not 1 to 1 for it's brainfuck equivalent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainIr {
	ChangeCell(
		i8,
		#[serde(skip_serializing_if = "Option::is_none")] Option<NonZero<i32>>,
	),
	MovePointer(i32),
	SetCell(
		u8,
		#[serde(skip_serializing_if = "Option::is_none")] Option<NonZero<i32>>,
	),
	SubCell(i32),
	FindZero(i32),
	InputIntoCell,
	OutputCell {
		value_offset: Option<NonZero<i8>>,
		offset: Option<NonZero<i32>>,
	},
	OutputChar(u8),
	OutputChars(Vec<u8>),
	MoveValueTo(u8, i32),
	TakeValueTo(u8, i32),
	FetchValueFrom(u8, i32),
	ReplaceValueFrom(u8, i32),
	ScaleValue(u8),
	DynamicLoop(Vec<Self>),
	IfNotZero(Vec<Self>),
	MemSet {
		value: u8,
		range: RangeInclusive<i32>,
	},
	DuplicateCell {
		// factor: i8,
		// indices: Vec<i32>,
		values: Vec<(i8, i32)>,
	},
}

impl BrainIr {
	#[must_use]
	pub const fn change_cell(value: i8) -> Self {
		Self::change_cell_at(value, 0)
	}

	#[must_use]
	pub const fn change_cell_at(value: i8, offset: i32) -> Self {
		Self::ChangeCell(value, NonZero::new(offset))
	}

	#[must_use]
	pub const fn move_pointer(offset: i32) -> Self {
		Self::MovePointer(offset)
	}

	#[must_use]
	pub const fn set_cell(value: u8) -> Self {
		Self::set_cell_at(value, 0)
	}

	#[must_use]
	pub const fn set_cell_at(value: u8, offset: i32) -> Self {
		Self::SetCell(value, NonZero::new(offset))
	}

	#[must_use]
	pub const fn sub_cell(offset: i32) -> Self {
		Self::SubCell(offset)
	}

	#[must_use]
	pub fn is_zeroing_cell(&self) -> bool {
		matches!(
			self,
			Self::SetCell(0, None)
				| Self::DynamicLoop(..)
				| Self::MoveValueTo(..)
				| Self::FindZero(..)
				| Self::SubCell(..)
				| Self::IfNotZero(..)
				| Self::DuplicateCell { .. }
		) || matches!(self, Self::MemSet { value: 0, range } if range.contains(&0))
	}

	#[must_use]
	pub const fn needs_nonzero_cell(&self) -> bool {
		matches!(
			self,
			Self::DynamicLoop(..)
				| Self::FindZero(..)
				| Self::MoveValueTo(..)
				| Self::SubCell(..)
				| Self::IfNotZero(..)
				| Self::DuplicateCell { .. }
		)
	}

	#[must_use]
	pub fn has_input(&self) -> bool {
		if let Some(children) = self.child_ops() {
			return children.iter().any(Self::has_input);
		}

		matches!(self, Self::InputIntoCell)
	}

	#[must_use]
	pub fn has_output(&self) -> bool {
		if let Some(children) = self.child_ops() {
			return children.iter().any(Self::has_output);
		}

		matches!(
			self,
			Self::OutputCell { .. } | Self::OutputChar(..) | Self::OutputChars(..)
		)
	}

	#[must_use]
	pub fn has_io(&self) -> bool {
		self.has_output() || self.has_input()
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
	pub const fn input_cell() -> Self {
		Self::InputIntoCell
	}

	#[must_use]
	pub const fn output_cell_at(offset: i32) -> Self {
		Self::output_offset_cell_at(0, offset)
	}

	#[must_use]
	pub const fn output_offset_cell_at(value: i8, offset: i32) -> Self {
		Self::OutputCell {
			value_offset: NonZero::new(value),
			offset: NonZero::new(offset),
		}
	}

	#[must_use]
	pub const fn output_cell() -> Self {
		Self::output_offset_cell_at(0, 0)
	}

	#[must_use]
	pub const fn output_offset_cell(value: i8) -> Self {
		Self::output_offset_cell_at(value, 0)
	}

	#[must_use]
	pub const fn output_char(c: u8) -> Self {
		Self::OutputChar(c)
	}

	#[must_use]
	pub fn output_chars(c: impl IntoIterator<Item = u8>) -> Self {
		Self::OutputChars(c.collect_to())
	}

	#[must_use]
	pub const fn fetch_value_from(value: u8, offset: i32) -> Self {
		Self::FetchValueFrom(value, offset)
	}

	#[must_use]
	pub const fn replace_value_from(value: u8, offset: i32) -> Self {
		Self::ReplaceValueFrom(value, offset)
	}

	#[must_use]
	pub const fn take_value_to(value: u8, offset: i32) -> Self {
		Self::TakeValueTo(value, offset)
	}

	#[must_use]
	pub const fn move_value_to(value: u8, offset: i32) -> Self {
		Self::MoveValueTo(value, offset)
	}

	#[must_use]
	pub const fn scale_value(value: u8) -> Self {
		Self::ScaleValue(value)
	}

	#[must_use]
	pub const fn find_zero(offset: i32) -> Self {
		Self::FindZero(offset)
	}

	#[must_use]
	pub const fn mem_set(value: u8, range: RangeInclusive<i32>) -> Self {
		Self::MemSet { value, range }
	}

	#[must_use]
	pub fn duplicate_cell(values: impl IntoIterator<Item = (i8, i32)>) -> Self {
		Self::DuplicateCell {
			values: values.collect_to(),
		}
	}

	#[must_use]
	pub const fn offset(&self) -> Option<i32> {
		match self {
			Self::ChangeCell(.., offset) | Self::SetCell(.., offset) => match offset {
				None => Some(0),
				Some(i) => Some(i.get()),
			},
			_ => None,
		}
	}

	#[must_use]
	pub const fn child_ops(&self) -> Option<&Vec<Self>> {
		match self {
			Self::DynamicLoop(ops) | Self::IfNotZero(ops) => Some(ops),
			_ => None,
		}
	}

	pub const fn child_ops_mut(&mut self) -> Option<&mut Vec<Self>> {
		match self {
			Self::DynamicLoop(ops) | Self::IfNotZero(ops) => Some(ops),
			_ => None,
		}
	}

	#[must_use]
	pub fn dynamic_loop(instrs: impl IntoIterator<Item = Self>) -> Self {
		Self::DynamicLoop(instrs.collect_to())
	}

	#[must_use]
	pub fn if_not_zero(instrs: impl IntoIterator<Item = Self>) -> Self {
		Self::IfNotZero(instrs.collect_to())
	}
}
