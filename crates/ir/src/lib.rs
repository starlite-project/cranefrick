#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod options;
mod output;
#[cfg(feature = "parse")]
mod parse;
mod sub;

use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

#[cfg(feature = "parse")]
pub use self::parse::*;
pub use self::{options::*, output::*, sub::*};

/// Mid-level intermediate representation. Not 1 to 1 for it's brainfuck equivalent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainIr {
	Boundary,
	ChangeCell(ValuedChangeCellOptions<i8>),
	SetCell(ValuedChangeCellOptions<u8>),
	SubCell(SubType),
	MovePointer(i32),
	FindZero(i32),
	InputIntoCell,
	Output(OutputOptions),
	MoveValueTo(FactoredChangeCellOptions<u8>),
	CopyValueTo(FactoredChangeCellOptions<u8>),
	TakeValueTo(FactoredChangeCellOptions<u8>),
	FetchValueFrom(FactoredChangeCellOptions<u8>),
	ReplaceValueFrom(FactoredChangeCellOptions<u8>),
	ScaleValue(u8),
	DynamicLoop(Vec<Self>),
	IfNotZero(Vec<Self>),
	SetRange(SetRangeOptions),
	SetManyCells(SetManyCellsOptions),
	DuplicateCell {
		values: Vec<FactoredChangeCellOptions<i8>>,
	},
}

impl BrainIr {
	#[must_use]
	pub const fn boundary() -> Self {
		Self::Boundary
	}

	#[must_use]
	pub const fn change_cell(value: i8) -> Self {
		Self::change_cell_at(value, 0)
	}

	#[must_use]
	pub const fn change_cell_at(value: i8, offset: i32) -> Self {
		Self::ChangeCell(ValuedChangeCellOptions::new(value, offset))
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
		Self::SetCell(ValuedChangeCellOptions::new(value, offset))
	}

	#[must_use]
	pub const fn sub_from_cell(value: u8, offset: i32) -> Self {
		Self::SubCell(SubType::FromCell(FactoredChangeCellOptions::new(
			value, offset,
		)))
	}

	#[must_use]
	pub const fn sub_cell_at(value: u8, offset: i32) -> Self {
		Self::SubCell(SubType::CellAt(FactoredChangeCellOptions::new(
			value, offset,
		)))
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
	pub fn output_cells(c: impl IntoIterator<Item = ValuedChangeCellOptions<i8>>) -> Self {
		Self::Output(OutputOptions::cells(c))
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
		Self::FetchValueFrom(FactoredChangeCellOptions::new(value, offset))
	}

	#[must_use]
	pub const fn replace_value_from(value: u8, offset: i32) -> Self {
		Self::ReplaceValueFrom(FactoredChangeCellOptions::new(value, offset))
	}

	#[must_use]
	pub const fn take_value_to(value: u8, offset: i32) -> Self {
		Self::TakeValueTo(FactoredChangeCellOptions::new(value, offset))
	}

	#[must_use]
	pub const fn move_value_to(value: u8, offset: i32) -> Self {
		Self::MoveValueTo(FactoredChangeCellOptions::new(value, offset))
	}

	#[must_use]
	pub const fn copy_value_to(value: u8, offset: i32) -> Self {
		Self::CopyValueTo(FactoredChangeCellOptions::new(value, offset))
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
	pub const fn set_range(value: u8, start: i32, end: i32) -> Self {
		Self::SetRange(SetRangeOptions::new(value, start, end))
	}

	#[must_use]
	pub fn set_many_cells(values: impl IntoIterator<Item = u8>, offset: i32) -> Self {
		Self::SetManyCells(SetManyCellsOptions::new(values, offset))
	}

	#[must_use]
	pub fn duplicate_cell(values: impl IntoIterator<Item = FactoredChangeCellOptions<i8>>) -> Self {
		Self::DuplicateCell {
			values: values.collect_to(),
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
		if let Some(children) = self.child_ops() {
			return children.iter().any(Self::has_io);
		}

		self.has_input() || self.has_output()
	}

	#[must_use]
	pub const fn offset(&self) -> Option<i32> {
		match self {
			Self::SetCell(options) => Some(options.offset()),
			Self::ChangeCell(options) | Self::Output(OutputOptions::Cell(options)) => {
				Some(options.offset())
			}
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
	pub fn is_zeroing_cell(&self) -> bool {
		match self {
			Self::DynamicLoop(..)
			| Self::MoveValueTo(..)
			| Self::FindZero(..)
			| Self::IfNotZero(..)
			| Self::SubCell(..)
			| Self::Boundary => true,
			Self::SetRange(options) => options.is_zeroing_cell(),
			Self::SetManyCells(options) => options.is_zeroing_cell(),
			Self::SetCell(options) => options.is_default(),
			_ => false,
		}
	}

	#[must_use]
	pub const fn needs_nonzero_cell(&self) -> bool {
		matches!(
			self,
			Self::DynamicLoop(..)
				| Self::FindZero(..)
				| Self::MoveValueTo(..)
				| Self::IfNotZero(..)
				| Self::SubCell(SubType::CellAt(..))
				| Self::DuplicateCell { .. }
				| Self::CopyValueTo(..)
		)
	}
}
