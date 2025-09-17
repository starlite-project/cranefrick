#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod compiler;
mod move_options;
mod output;

use std::{num::NonZero, ops::RangeInclusive};

use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

pub use self::{compiler::*, move_options::*, output::*};

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
	Output(OutputOptions),
	MoveValueTo(MoveOptions),
	CopyValueTo(MoveOptions),
	TakeValueTo(MoveOptions),
	FetchValueFrom(MoveOptions),
	ReplaceValueFrom(MoveOptions),
	ScaleValue(u8),
	DynamicLoop(Vec<Self>),
	IfNotZero(Vec<Self>),
	SetRange {
		value: u8,
		range: RangeInclusive<i32>,
	},
	SetManyCells {
		values: Vec<u8>,
		start: Option<NonZero<i32>>,
	},
	DuplicateCell {
		values: Vec<MoveOptions<i8>>,
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
		) || matches!(self, Self::SetRange { value: 0, range } if range.contains(&0))
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
				| Self::CopyValueTo(..)
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

		matches!(self, Self::Output(..))
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
		Self::Output(OutputOptions::cell(value, offset))
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
		Self::Output(OutputOptions::char(c))
	}

	#[must_use]
	pub fn output_str(c: impl IntoIterator<Item = u8>) -> Self {
		Self::Output(OutputOptions::str(c))
	}

	#[must_use]
	pub const fn fetch_value_from(value: u8, offset: i32) -> Self {
		Self::FetchValueFrom(MoveOptions::new(value, offset))
	}

	#[must_use]
	pub const fn replace_value_from(value: u8, offset: i32) -> Self {
		Self::ReplaceValueFrom(MoveOptions::new(value, offset))
	}

	#[must_use]
	pub const fn take_value_to(value: u8, offset: i32) -> Self {
		Self::TakeValueTo(MoveOptions::new(value, offset))
	}

	#[must_use]
	pub const fn move_value_to(value: u8, offset: i32) -> Self {
		Self::MoveValueTo(MoveOptions::new(value, offset))
	}

	#[must_use]
	pub const fn copy_value_to(value: u8, offset: i32) -> Self {
		Self::CopyValueTo(MoveOptions::new(value, offset))
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
	pub const fn set_range(value: u8, range: RangeInclusive<i32>) -> Self {
		Self::SetRange { value, range }
	}

	#[must_use]
	pub fn set_many_cells(values: impl IntoIterator<Item = u8>, offset: i32) -> Self {
		Self::SetManyCells {
			values: values.collect_to(),
			start: NonZero::new(offset),
		}
	}

	#[must_use]
	pub fn duplicate_cell(values: impl IntoIterator<Item = (i8, i32)>) -> Self {
		Self::DuplicateCell {
			values: values
				.into_iter()
				.map(|(factor, offset)| MoveOptions::new(factor, offset))
				.collect(),
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
