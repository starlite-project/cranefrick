#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod compiler;

use std::num::NonZeroI32;

use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

pub use self::compiler::*;

/// Mid-level intermediate representation. Not 1 to 1 for it's brainfuck equivalent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainIr {
	ChangeCell(
		i8,
		#[serde(skip_serializing_if = "Option::is_none")] Option<NonZeroI32>,
	),
	MovePointer(i32),
	SetCell(
		u8,
		#[serde(skip_serializing_if = "Option::is_none")] Option<NonZeroI32>,
	),
	SubCell(i32),
	FindZero(i32),
	InputIntoCell,
	OutputCurrentCell,
	OutputChar(u8),
	OutputChars(Vec<u8>),
	MoveValueTo(u8, i32),
	TakeValueTo(u8, i32),
	FetchValueFrom(u8, i32),
	ReplaceValueFrom(u8, i32),
	ScaleValue(u8),
	DynamicLoop(Vec<Self>),
	IfNz(Vec<Self>),
}

impl BrainIr {
	#[must_use]
	pub const fn change_cell(value: i8) -> Self {
		Self::change_cell_at(value, 0)
	}

	#[must_use]
	pub const fn change_cell_at(value: i8, offset: i32) -> Self {
		Self::ChangeCell(value, NonZeroI32::new(offset))
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
		Self::SetCell(value, NonZeroI32::new(offset))
	}

	#[must_use]
	pub const fn sub_cell(offset: i32) -> Self {
		Self::SubCell(offset)
	}

	#[must_use]
	pub const fn is_zeroing_cell(&self) -> bool {
		matches!(
			self,
			Self::SetCell(0, None)
				| Self::DynamicLoop(..)
				| Self::MoveValueTo(..)
				| Self::FindZero(..)
				| Self::SubCell(..)
				| Self::IfNz(..)
		)
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
	pub const fn output_current_cell() -> Self {
		Self::OutputCurrentCell
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
			Self::DynamicLoop(ops) | Self::IfNz(ops) => Some(ops),
			_ => None,
		}
	}

	pub const fn child_ops_mut(&mut self) -> Option<&mut Vec<Self>> {
		match self {
			Self::DynamicLoop(ops) | Self::IfNz(ops) => Some(ops),
			_ => None,
		}
	}

	#[must_use]
	pub fn dynamic_loop(instrs: impl IntoIterator<Item = Self>) -> Self {
		Self::DynamicLoop(instrs.collect_to())
	}

	#[must_use]
	pub fn if_nz(instrs: impl IntoIterator<Item = Self>) -> Self {
		Self::IfNz(instrs.collect_to())
	}
}
