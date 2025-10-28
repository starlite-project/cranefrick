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
	ChangeCell(ValuedChangeCellOptions<i8>),
	SetCell(ValuedChangeCellOptions<u8>),
	SubCell(SubOptions),
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
		Self::SubCell(SubOptions::FromCell(FactoredChangeCellOptions::new(
			value, offset,
		)))
	}

	#[must_use]
	pub const fn sub_cell_at(value: u8, offset: i32) -> Self {
		Self::SubCell(SubOptions::CellAt(FactoredChangeCellOptions::new(
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
			matches!(self, Self::InputIntoCell)
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
				Self::MovePointer(offset) => movement = movement.wrapping_add(*offset),
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
	pub fn is_zeroing_cell(&self) -> bool {
		match self {
			Self::DynamicLoop(..)
			| Self::MoveValueTo(..)
			| Self::FindZero(..)
			| Self::IfNotZero(..)
			| Self::SubCell(SubOptions::CellAt(..))
			| Self::DuplicateCell { .. } => true,
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
				| Self::SubCell(SubOptions::CellAt(..))
				| Self::DuplicateCell { .. }
				| Self::CopyValueTo(..)
		)
	}
}

impl Display for BrainIr {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Boundary => f.write_str("boundary")?,
			Self::ChangeCell(change_options) => {
				match change_options.into_parts() {
					(a, 0) => {
						f.write_str("change_cell(")?;
						Display::fmt(&a, f)?;
					}
					(a, x) => {
						f.write_str("change_cell_at(")?;
						Display::fmt(&a, f)?;
						f.write_str(", ")?;
						Display::fmt(&x, f)?;
					}
				}
				f.write_char(')')?;
			}
			Self::SetCell(set_options) => match set_options.into_parts() {
				(0, 0) => f.write_str("clear_cell")?,
				(0, x) => {
					f.write_str("clear_cell_at(")?;
					Display::fmt(&x, f)?;
					f.write_char(')')?;
				}
				(a, 0) => {
					f.write_str("set_cell(")?;
					Display::fmt(&a, f)?;
					f.write_char(')')?;
				}
				(a, x) => {
					f.write_str("set_cell_at(")?;
					Display::fmt(&a, f)?;
					f.write_str(", ")?;
					Display::fmt(&x, f)?;
					f.write_char(')')?;
				}
			},
			Self::SubCell(SubOptions::CellAt(sub_at_options)) => {
				f.write_str("sub_cell_at(")?;
				match sub_at_options.into_parts() {
					(1, x) => Display::fmt(&x, f)?,
					(a, x) => {
						Display::fmt(&a, f)?;
						f.write_str(", ")?;
						Display::fmt(&x, f)?;
					}
				}
				f.write_char(')')?;
			}
			Self::SubCell(SubOptions::FromCell(sub_from_options)) => {
				f.write_str("sub_from_cell(")?;
				match sub_from_options.into_parts() {
					(1, x) => Display::fmt(&x, f)?,
					(a, x) => {
						Display::fmt(&a, f)?;
						f.write_str(", ")?;
						Display::fmt(&x, f)?;
					}
				}
				f.write_char(')')?;
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
			Self::InputIntoCell => f.write_str("input")?,
			Self::Output(output_options) => Display::fmt(&output_options, f)?,
			Self::MoveValueTo(move_options) => {
				f.write_str("move_value_to(")?;
				write_shift_options(*move_options, f)?;
			}
			Self::CopyValueTo(copy_options) => {
				f.write_str("copy_value_to(")?;
				write_shift_options(*copy_options, f)?;
			}
			Self::TakeValueTo(take_options) => {
				f.write_str("take_value_to(")?;
				write_shift_options(*take_options, f)?;
			}
			Self::FetchValueFrom(fetch_options) => {
				f.write_str("fetch_value_from(")?;
				write_shift_options(*fetch_options, f)?;
			}
			Self::ReplaceValueFrom(replace_options) => {
				f.write_str("replace_value_from(")?;
				write_shift_options(*replace_options, f)?;
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
				for value in set_many_options.values() {
					if !is_first {
						f.write_str(", ")?;
					}

					Display::fmt(value, f)?;
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

					f.write_char('(')?;
					Display::fmt(&option.factor(), f)?;
					f.write_str(", ")?;
					Display::fmt(&option.offset(), f)?;
					f.write_char(')')?;

					is_first = false;
				}
			}
			_ => f.write_str("unknown_instr")?,
		}

		Ok(())
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

fn write_shift_options(options: FactoredChangeCellOptions<u8>, f: &mut Formatter<'_>) -> FmtResult {
	match options.into_parts() {
		(1, x) => Display::fmt(&x, f)?,
		(a, x) => {
			Display::fmt(&a, f)?;
			f.write_str(", ")?;
			Display::fmt(&x, f)?;
		}
	}

	f.write_char(')')
}
