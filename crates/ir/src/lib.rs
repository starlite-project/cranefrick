#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(unreachable_patterns)]
#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod options;
mod output;
#[cfg(feature = "parse")]
mod parse;

use alloc::vec::Vec;
use core::fmt::{Display, Formatter, Result as FmtResult, Write as _};

use frick_utils::{IntoIteratorExt as _, IteratorExt as _};
use serde::{Deserialize, Serialize};

#[cfg(feature = "parse")]
pub use self::parse::*;
pub use self::{options::*, output::*};

/// Mid-level intermediate representation. Not 1 to 1 for it's brainfuck equivalent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainIr {
	Boundary,
	ChangeCell(ValuedOffsetCellOptions<i8>),
	SetCell(ValuedOffsetCellOptions<u8>),
	SubCell(SubOptions),
	MovePointer(i32),
	FindZero(i32),
	InputIntoCell(ValuedOffsetCellOptions<i8>),
	Output(OutputOptions),
	MoveValueTo(FactoredOffsetCellOptions<u8>),
	TakeValueTo(FactoredOffsetCellOptions<u8>),
	FetchValueFrom(FactoredOffsetCellOptions<u8>),
	ReplaceValueFrom(FactoredOffsetCellOptions<u8>),
	ScaleValue(u8),
	DynamicLoop(Vec<Self>),
	IfNotZero(Vec<Self>),
	ChangeManyCells(ChangeManyCellsOptions),
	SetRange(SetRangeOptions),
	SetManyCells(SetManyCellsOptions),
	DuplicateCell {
		values: Vec<FactoredOffsetCellOptions<i8>>,
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
		Self::ChangeCell(ValuedOffsetCellOptions::new(value, offset))
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
		Self::SetCell(ValuedOffsetCellOptions::new(value, offset))
	}

	#[must_use]
	pub const fn sub_from_cell(value: u8, offset: i32) -> Self {
		Self::SubCell(SubOptions::FromCell(FactoredOffsetCellOptions::new(
			value, offset,
		)))
	}

	#[must_use]
	pub const fn sub_cell_at(value: u8, offset: i32) -> Self {
		Self::SubCell(SubOptions::CellAt(FactoredOffsetCellOptions::new(
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
	pub const fn input_into_cell() -> Self {
		Self::input_offset_into_cell(0)
	}

	#[must_use]
	pub const fn input_into_cell_at(offset: i32) -> Self {
		Self::input_offset_into_cell_at(0, offset)
	}

	#[must_use]
	pub const fn input_offset_into_cell(value: i8) -> Self {
		Self::input_offset_into_cell_at(value, 0)
	}

	#[must_use]
	pub const fn input_offset_into_cell_at(value: i8, offset: i32) -> Self {
		Self::InputIntoCell(ValuedOffsetCellOptions::new(value, offset))
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
	pub fn output_cells(c: impl IntoIterator<Item = ValuedOffsetCellOptions<i8>>) -> Self {
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
		Self::FetchValueFrom(FactoredOffsetCellOptions::new(value, offset))
	}

	#[must_use]
	pub const fn replace_value_from(value: u8, offset: i32) -> Self {
		Self::ReplaceValueFrom(FactoredOffsetCellOptions::new(value, offset))
	}

	#[must_use]
	pub const fn take_value_to(value: u8, offset: i32) -> Self {
		Self::TakeValueTo(FactoredOffsetCellOptions::new(value, offset))
	}

	#[must_use]
	pub const fn move_value_to(value: u8, offset: i32) -> Self {
		Self::MoveValueTo(FactoredOffsetCellOptions::new(value, offset))
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
	pub fn change_many_cells(values: impl IntoIterator<Item = i8>, offset: i32) -> Self {
		Self::ChangeManyCells(ChangeManyCellsOptions::new(values, offset))
	}

	#[must_use]
	pub fn set_many_cells(values: impl IntoIterator<Item = u8>, offset: i32) -> Self {
		Self::SetManyCells(SetManyCellsOptions::new(values, offset))
	}

	#[must_use]
	pub fn duplicate_cell(values: impl IntoIterator<Item = FactoredOffsetCellOptions<i8>>) -> Self {
		Self::DuplicateCell {
			values: values.into_iter().sorted_by_key(|x| x.offset()).collect(),
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
			children.iter().any(Self::has_input)
		} else {
			matches!(self, Self::InputIntoCell(..))
		}
	}

	#[must_use]
	pub fn has_output(&self) -> bool {
		if let Some(children) = self.child_ops() {
			children.iter().any(Self::has_output)
		} else {
			matches!(self, Self::Output(..))
		}
	}

	#[must_use]
	pub fn has_io(&self) -> bool {
		if let Some(children) = self.child_ops() {
			children.iter().any(Self::has_io)
		} else {
			self.has_input() || self.has_output()
		}
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
	pub fn loop_has_movement(&self) -> Option<bool> {
		let child_ops = self.child_ops()?;

		let mut movement = 0i32;

		for op in child_ops {
			match op {
				&Self::MovePointer(offset) => movement = movement.wrapping_add(offset),
				Self::TakeValueTo(options) => {
					movement = movement.wrapping_add(options.offset());
				}
				Self::DynamicLoop(..) | Self::IfNotZero(..) => return None,
				_ => {}
			}
		}

		Some(!matches!(movement, 0))
	}

	#[must_use]
	pub fn is_clobbering_cell(&self) -> bool {
		self.is_zeroing_cell()
			|| match self {
				Self::ReplaceValueFrom(..) => true,
				Self::SetRange(set_range_options) => set_range_options.is_clobbering_cell(),
				Self::SetManyCells(set_many_options) => set_many_options.is_clobbering_cell(),
				Self::SetCell(set_options) => !set_options.is_offset(),
				Self::InputIntoCell(input_options) => !input_options.is_offset(),
				_ => false,
			}
	}

	#[must_use]
	pub fn is_zeroing_cell(&self) -> bool {
		match self {
			Self::DynamicLoop(..)
			| Self::MoveValueTo(..)
			| Self::FindZero(..)
			| Self::IfNotZero(..)
			| Self::SubCell(SubOptions::CellAt(..)) => true,
			Self::SetRange(set_range_options) => set_range_options.is_zeroing_cell(),
			Self::SetManyCells(set_many_options) => set_many_options.is_zeroing_cell(),
			Self::SetCell(set_options) => set_options.is_default(),
			Self::DuplicateCell { values } => !values.iter().any(|x| matches!(x.offset(), 0)),
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
				| Self::SubCell(SubOptions::CellAt(..))
				| Self::DuplicateCell { .. }
		)
	}
}

impl Display for BrainIr {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Boundary => f.write_str("boundary")?,
			Self::ChangeCell(change_options) => {
				f.write_str("change_cell")?;
				Display::fmt(&change_options, f)?;
			}
			Self::SetCell(set_options) => {
				f.write_str("set_cell")?;
				Display::fmt(&set_options, f)?;
			}
			Self::SubCell(SubOptions::CellAt(sub_at_options)) => {
				f.write_str("sub_cell_at")?;
				Display::fmt(&sub_at_options, f)?;
			}
			Self::SubCell(SubOptions::FromCell(sub_from_options)) => {
				f.write_str("sub_from_cell")?;
				Display::fmt(&sub_from_options, f)?;
			}
			Self::MovePointer(offset) => {
				f.write_str("move_pointer(")?;
				Display::fmt(&offset, f)?;
				f.write_char(')')?;
			}
			Self::FindZero(offset) => {
				f.write_str("find_zero(")?;
				Display::fmt(&offset, f)?;
				f.write_char(')')?;
			}
			Self::InputIntoCell(input_options) => {
				f.write_str("input")?;
				Display::fmt(&input_options, f)?;
			}
			Self::Output(output_options) => Display::fmt(&output_options, f)?,
			Self::MoveValueTo(move_options) => {
				f.write_str("move_value_to")?;
				Display::fmt(&move_options, f)?;
			}
			Self::TakeValueTo(take_options) => {
				f.write_str("take_value_to")?;
				Display::fmt(&take_options, f)?;
			}
			Self::FetchValueFrom(fetch_options) => {
				f.write_str("fetch_value_from")?;
				Display::fmt(&fetch_options, f)?;
			}
			Self::ReplaceValueFrom(replace_options) => {
				f.write_str("replace_value_from")?;
				Display::fmt(&replace_options, f)?;
			}
			Self::ScaleValue(factor) => {
				f.write_str("scale_value(")?;
				Display::fmt(&factor, f)?;
				f.write_char(')')?;
			}
			Self::DynamicLoop(ops) => {
				f.write_str("dynamic_loop(")?;
				Display::fmt(&ops.len(), f)?;
				f.write_char(')')?;
			}
			Self::IfNotZero(ops) => {
				f.write_str("if_not_zero(")?;
				Display::fmt(&ops.len(), f)?;
				f.write_char(')')?;
			}
			Self::SetRange(set_range_options) => {
				f.write_str("set_range(")?;
				Display::fmt(&set_range_options.value(), f)?;
				f.write_str(", ")?;
				Display::fmt(&set_range_options.start(), f)?;
				f.write_str("..")?;
				Display::fmt(&set_range_options.end(), f)?;
				f.write_char(')')?;
			}
			Self::SetManyCells(set_many_options) => {
				f.write_str("set_many_cells((")?;
				let mut is_first = true;
				for option in set_many_options {
					if !is_first {
						f.write_str(", ")?;
					}

					Display::fmt(&option, f)?;
					is_first = false;
				}

				f.write_str(") @ ")?;
				Display::fmt(&set_many_options.start(), f)?;
				f.write_char(')')?;
			}
			Self::DuplicateCell { values } => {
				f.write_str("duplicate_cell(")?;
				let mut is_first = true;
				for option in values {
					if !is_first {
						f.write_str(", ")?;
					}

					Display::fmt(option, f)?;
					is_first = false;
				}
			}
			_ => f.write_str("unknown_instr")?,
		}

		Ok(())
	}
}

impl From<ChangeManyCellsOptions> for BrainIr {
	fn from(value: ChangeManyCellsOptions) -> Self {
		Self::ChangeManyCells(value)
	}
}

impl From<OutputOptions> for BrainIr {
	fn from(value: OutputOptions) -> Self {
		Self::Output(value)
	}
}

impl From<SetManyCellsOptions> for BrainIr {
	fn from(value: SetManyCellsOptions) -> Self {
		Self::SetManyCells(value)
	}
}

impl From<SetRangeOptions> for BrainIr {
	fn from(value: SetRangeOptions) -> Self {
		Self::SetRange(value)
	}
}
