#![allow(clippy::trivially_copy_pass_by_ref)]

mod loops;
mod misc;
mod sort;

use std::{cmp, iter};

use frick_ir::{BrainIr, ChangeCellOptions, OutputOptions, SetManyCellsOptions, SubOptions};
use frick_utils::{Convert as _, GetOrZero as _, InsertOrPush as _};

pub use self::{loops::*, misc::*, sort::*};
use crate::inner::{Change, utils::calculate_ptr_movement};

pub const fn optimize_consecutive_instructions(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(a), BrainIr::ChangeCell(b)] if a.offset() == b.offset() => {
			Some(if a.value() == -b.value() {
				Change::remove()
			} else {
				Change::replace(BrainIr::change_cell_at(
					a.value().wrapping_add(b.value()),
					a.offset(),
				))
			})
		}
		[BrainIr::MovePointer(a), BrainIr::MovePointer(b)] => Some(if *a == -(*b) {
			Change::remove()
		} else {
			Change::replace(BrainIr::move_pointer(a.wrapping_add(*b)))
		}),
		_ => None,
	}
}

pub fn optimize_sets(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(a), BrainIr::SetCell(b)] if a.offset() == b.offset() => {
			Some(Change::remove_offset(0))
		}
		[BrainIr::ChangeCell(a), BrainIr::SetCell(b)] if a.offset() == b.offset() => {
			Some(Change::remove_offset(0))
		}
		[BrainIr::SetCell(a), BrainIr::ChangeCell(b)] if a.offset() == b.offset() => {
			Some(Change::replace(BrainIr::set_cell_at(
				a.value().wrapping_add_signed(b.value()),
				a.offset(),
			)))
		}
		[l, BrainIr::ChangeCell(options)] if l.is_zeroing_cell() && !options.is_offset() => {
			Some(Change::swap([
				l.clone(),
				BrainIr::set_cell(options.value() as u8),
			]))
		}
		[BrainIr::SetCell(options), BrainIr::InputIntoCell] if !options.is_offset() => {
			Some(Change::remove_offset(0))
		}
		[BrainIr::ChangeCell(options), BrainIr::InputIntoCell] if !options.is_offset() => {
			Some(Change::remove_offset(0))
		}
		[l, BrainIr::SetCell(options)] if options.is_default() && l.is_zeroing_cell() => {
			Some(Change::remove_offset(1))
		}
		[
			BrainIr::SetCell(set_cell_options),
			BrainIr::SetRange(set_range_options),
		] => {
			let range = set_range_options.range();
			let x = set_cell_options.offset();

			range.contains(&x).then(|| Change::remove_offset(0))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::SetManyCells(set_many_options),
		] if set_many_options.range().contains(&change_options.offset()) => {
			Some(Change::remove_offset(0))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::SetRange(set_range_options),
		] if set_range_options.range().contains(&change_options.offset()) => {
			Some(Change::remove_offset(0))
		}
		_ => None,
	}
}

pub fn remove_unreachable_loops(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[a, b] if a.is_zeroing_cell() && b.needs_nonzero_cell() => Some(Change::remove_offset(1)),
		_ => None,
	}
}

pub const fn remove_noop_instructions(ops: [&BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(options)] if matches!(options.value(), 0) => Some(Change::remove()),
		[BrainIr::MovePointer(0)] => Some(Change::remove()),
		_ => None,
	}
}

pub fn fix_boundary_instructions(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::Boundary, BrainIr::ChangeCell(options)] => Some(Change::swap([
			BrainIr::boundary(),
			BrainIr::set_cell_at(options.value() as u8, options.offset()),
		])),
		[l, BrainIr::Boundary] if !l.has_output() => Some(Change::remove_offset(0)),
		[BrainIr::Boundary, BrainIr::Output(OutputOptions::Cell(..))] => Some(Change::swap([
			BrainIr::boundary(),
			BrainIr::output_char(0),
			BrainIr::set_cell(0),
		])),
		[
			BrainIr::Boundary,
			BrainIr::DynamicLoop(..)
			| BrainIr::CopyValueTo(..)
			| BrainIr::MoveValueTo(..)
			| BrainIr::FetchValueFrom(..)
			| BrainIr::ReplaceValueFrom(..),
		] => Some(Change::remove_offset(1)),
		[BrainIr::Boundary, BrainIr::TakeValueTo(take_options)] => Some(Change::swap([
			BrainIr::boundary(),
			BrainIr::move_pointer(take_options.offset()),
		])),
		_ => None,
	}
}

pub fn optimize_initial_sets(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::Boundary,
			BrainIr::SetCell(set_options),
			BrainIr::ChangeCell(change_options),
		] => {
			let x = set_options.offset();
			let y = change_options.offset();

			if x == y {
				return None;
			}

			let range = (x..y).skip(1);

			let mut swap = Vec::with_capacity(range.len() + 3);

			swap.extend(
				[
					BrainIr::boundary(),
					BrainIr::set_cell_at(set_options.value(), set_options.offset()),
				]
				.into_iter()
				.chain(
					range
						.map(BrainIr::clear_cell_at)
						.chain(iter::once(BrainIr::set_cell_at(
							change_options.value() as u8,
							y,
						))),
				),
			);

			Some(Change::swap(swap))
		}
		[
			BrainIr::Boundary,
			BrainIr::SetManyCells(set_many_options),
			BrainIr::ChangeCell(change_options),
		] => {
			let x = change_options.offset();
			let range = set_many_options.range();

			let range = range.end..x;

			let mut swap = Vec::with_capacity(range.len() + 3);

			swap.extend(
				[
					BrainIr::boundary(),
					BrainIr::SetManyCells(set_many_options.clone()),
				]
				.into_iter()
				.chain(
					range
						.map(BrainIr::clear_cell_at)
						.chain(iter::once(BrainIr::set_cell_at(
							change_options.value() as u8,
							x,
						))),
				),
			);

			Some(Change::swap(swap))
		}
		[
			BrainIr::Boundary,
			BrainIr::MovePointer(y),
			BrainIr::ChangeCell(change_options),
		] => Some(Change::swap([
			BrainIr::boundary(),
			BrainIr::move_pointer(*y),
			BrainIr::SetCell(ChangeCellOptions::new(
				change_options.value() as u8,
				change_options.offset(),
			)),
		])),
		_ => None,
	}
}

pub fn add_offsets(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::MovePointer(x),
			BrainIr::ChangeCell(change_options),
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			BrainIr::change_cell_at(
				change_options.value(),
				x.wrapping_add(change_options.offset()),
			),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::MovePointer(x),
			BrainIr::SetCell(set_options),
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			BrainIr::set_cell_at(set_options.value(), x.wrapping_add(set_options.offset())),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::MovePointer(x),
			BrainIr::SetRange(options),
			BrainIr::MovePointer(y),
		] => {
			let start = options.start().wrapping_add(*x);
			let end = options.end().wrapping_add(*y);

			let set_range_instr = BrainIr::set_range(options.value(), start, end);

			Some(if *x == -y {
				Change::replace(set_range_instr)
			} else {
				Change::swap([set_range_instr, BrainIr::move_pointer(x.wrapping_add(*y))])
			})
		}
		_ => None,
	}
}

pub fn remove_offsets(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(set_options), BrainIr::MovePointer(x)] if set_options.offset() == *x => {
			Some(Change::swap([
				BrainIr::move_pointer(*x),
				BrainIr::set_cell(set_options.value()),
			]))
		}
		[BrainIr::ChangeCell(change_options), BrainIr::MovePointer(x)]
			if change_options.offset() == *x =>
		{
			Some(Change::swap([
				BrainIr::move_pointer(*x),
				BrainIr::change_cell(change_options.value()),
			]))
		}
		[
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::MovePointer(y),
		] if output_options.offset() == *y => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::output_offset_cell(output_options.value()),
		])),
		[
			BrainIr::Output(OutputOptions::Cells(output_options)),
			BrainIr::MovePointer(y),
		] if output_options.iter().all(|x| x.offset() == *y) => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::output_cells(
				output_options
					.iter()
					.copied()
					.map(|x| ChangeCellOptions::new(x.value(), 0)),
			),
		])),
		_ => None,
	}
}

pub fn optimize_move_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::TakeValueTo(take_options), BrainIr::MovePointer(y)] => Some(Change::swap([
			BrainIr::move_value_to(take_options.factor(), take_options.offset()),
			BrainIr::move_pointer(take_options.offset().wrapping_add(*y)),
		])),
		_ => None,
	}
}

pub fn optimize_move_value_from_duplicate_cells(ops: [&BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::DuplicateCell { values }] if matches!(values.len(), 1) => {
			let data = values.first().copied()?;

			let value = data.factor();
			let index = data.offset();

			if value.is_negative() {
				None
			} else {
				Some(Change::replace(BrainIr::move_value_to(
					value.unsigned_abs(),
					index,
				)))
			}
		}
		_ => None,
	}
}

pub const fn optimize_take_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::MoveValueTo(options), BrainIr::MovePointer(y)] if options.offset() == *y => Some(
			Change::replace(BrainIr::take_value_to(options.factor(), options.offset())),
		),
		_ => None,
	}
}

pub fn optimize_fetch_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::MovePointer(x), BrainIr::TakeValueTo(take_options)] => Some(Change::swap([
			BrainIr::move_pointer(x.wrapping_add(take_options.offset())),
			BrainIr::fetch_value_from(take_options.factor(), -take_options.offset()),
		])),
		[BrainIr::MovePointer(x), BrainIr::MoveValueTo(move_options)]
			if *x == -move_options.offset() =>
		{
			Some(Change::swap([
				BrainIr::fetch_value_from(move_options.factor(), *x),
				BrainIr::move_pointer(*x),
			]))
		}
		_ => None,
	}
}

pub fn optimize_replace_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[l, BrainIr::FetchValueFrom(options)] if l.is_zeroing_cell() => Some(Change::swap([
			l.clone(),
			BrainIr::ReplaceValueFrom(*options),
		])),
		[BrainIr::SetCell(set_options), BrainIr::ReplaceValueFrom(..)]
			if !set_options.is_offset() =>
		{
			Some(Change::remove_offset(0))
		}
		_ => None,
	}
}

pub fn optimize_copy_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::DuplicateCell { values },
			BrainIr::ReplaceValueFrom(options),
		] => {
			if !values
				.iter()
				.any(|v| v.offset() == options.offset() && matches!(v.factor(), 1))
			{
				return None;
			}

			if !matches!(values.len(), 2) {
				return None;
			}

			let other_move_options = {
				let offset = values
					.iter()
					.position(|x| x.offset() != options.offset() && x.factor().is_positive())?;

				values.get(offset).copied()?
			};

			Some(Change::swap([
				BrainIr::copy_value_to(
					other_move_options.factor() as u8,
					other_move_options.offset(),
				),
				BrainIr::FetchValueFrom(*options),
			]))
		}
		_ => None,
	}
}

pub fn optimize_writes(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(set_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
		] if output_options.offset() == set_options.offset() => Some(Change::swap([
			BrainIr::output_char(
				set_options
					.value()
					.wrapping_add_signed(output_options.value()),
			),
			BrainIr::set_cell_at(set_options.value(), set_options.offset()),
		])),
		[l, BrainIr::Output(OutputOptions::Cell(options))]
			if l.is_zeroing_cell() && !options.is_offset() =>
		{
			Some(Change::swap([l.clone(), BrainIr::output_char(0)]))
		}
		[
			BrainIr::Output(OutputOptions::Char(x)),
			BrainIr::Output(OutputOptions::Char(y)),
		] => Some(Change::replace(BrainIr::output_str([*x, *y]))),
		[
			BrainIr::Output(OutputOptions::Str(chars)),
			BrainIr::Output(OutputOptions::Char(c)),
		] => Some(Change::replace(BrainIr::output_str(
			chars.iter().copied().chain(iter::once(*c)),
		))),
		[
			BrainIr::Output(OutputOptions::Char(c)),
			BrainIr::Output(OutputOptions::Str(chars)),
		] => Some(Change::replace(BrainIr::output_str(
			iter::once(*c).chain(chars.iter().copied()),
		))),
		[
			BrainIr::Output(OutputOptions::Str(a)),
			BrainIr::Output(OutputOptions::Str(b)),
		] => Some(Change::replace(BrainIr::output_str(
			a.iter().copied().chain(b.iter().copied()),
		))),
		[
			BrainIr::Output(OutputOptions::Cell(x)),
			BrainIr::Output(OutputOptions::Cell(y)),
		] => Some(Change::replace(BrainIr::output_cells([*x, *y]))),
		[
			BrainIr::Output(OutputOptions::Cell(x)),
			BrainIr::Output(OutputOptions::Cells(other)),
		] => Some(Change::replace(BrainIr::output_cells(
			iter::once(*x).chain(other.iter().copied()),
		))),
		[
			BrainIr::Output(OutputOptions::Cells(other)),
			BrainIr::Output(OutputOptions::Cell(x)),
		] => Some(Change::replace(BrainIr::output_cells(
			other.iter().copied().chain(iter::once(*x)),
		))),
		[
			BrainIr::Output(OutputOptions::Cells(a)),
			BrainIr::Output(OutputOptions::Cells(b)),
		] => Some(Change::replace(BrainIr::output_cells(
			a.iter().copied().chain(b.iter().copied()),
		))),
		[
			BrainIr::SetCell(set_options),
			BrainIr::Output(OutputOptions::Cells(output_options)),
		] if output_options
			.iter()
			.all(|x| x.offset() == set_options.offset()) =>
		{
			let mut chars = Vec::with_capacity(output_options.len());

			for value_offset in output_options.iter().map(|x| x.value()) {
				chars.push(set_options.value().wrapping_add_signed(value_offset));
			}

			Some(Change::swap([
				BrainIr::output_str(chars),
				BrainIr::set_cell_at(set_options.value(), set_options.offset()),
			]))
		}
		[
			BrainIr::SetManyCells(set_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
		] => {
			let char_at = set_options.value_at(output_options.offset())?;

			Some(Change::swap([
				BrainIr::output_char(char_at),
				BrainIr::set_many_cells(set_options.values().iter().copied(), set_options.start()),
			]))
		}
		[
			BrainIr::SetManyCells(set_options),
			BrainIr::Output(OutputOptions::Cells(output_options)),
		] => {
			if !output_options
				.iter()
				.all(|x| set_options.value_at(x.offset()).is_some())
			{
				return None;
			}

			let mut str = Vec::with_capacity(output_options.len());

			for opt in output_options {
				let current_value = set_options.value_at(opt.offset())?;

				str.push(current_value);
			}

			Some(Change::swap([
				BrainIr::output_str(str),
				BrainIr::set_many_cells(set_options.values().iter().copied(), set_options.start()),
			]))
		}
		[
			change @ (BrainIr::SetCell(..) | BrainIr::ChangeCell(..)),
			out @ BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
		] => Some(Change::swap([out.clone(), change.clone()])),
		_ => None,
	}
}

pub fn optimize_offset_writes(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(a),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::ChangeCell(b),
		] if !a.is_offset() && !output_options.is_offset() && !b.is_offset() => {
			if a.value() == -b.value() {
				Some(Change::replace(BrainIr::output_offset_cell(
					a.value().wrapping_add(output_options.value()),
				)))
			} else if output_options.is_default() {
				Some(Change::swap([
					BrainIr::output_offset_cell(a.value()),
					BrainIr::change_cell(a.value().wrapping_add(b.value())),
				]))
			} else {
				Some(Change::swap([
					BrainIr::change_cell(a.value().wrapping_add(b.value())),
					BrainIr::output_offset_cell(output_options.value().wrapping_sub(b.value())),
				]))
			}
		}
		[
			BrainIr::MovePointer(x),
			BrainIr::Output(OutputOptions::Cell(options)),
			BrainIr::MovePointer(y),
		] if options.is_default() => Some(Change::swap([
			BrainIr::output_cell_at(*x),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::MovePointer(x),
			out @ BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			out.clone(),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::SetCell(set_options),
		] if !change_options.is_offset()
			&& !output_options.is_offset()
			&& !set_options.is_offset() =>
		{
			Some(Change::swap([
				BrainIr::output_offset_cell(
					change_options.value().wrapping_add(output_options.value()),
				),
				BrainIr::set_cell(set_options.value()),
			]))
		}
		[
			BrainIr::ChangeCell(a),
			BrainIr::Output(OutputOptions::Cells(options)),
			BrainIr::ChangeCell(b),
		] => {
			let x = a.offset();
			let y = b.offset();

			if x != -y {
				return None;
			}

			let mut output = Vec::with_capacity(options.len());

			for option in options {
				if option.offset() == x {
					output.push(ChangeCellOptions::new(
						option.value().wrapping_add(a.value()),
						x,
					));
				} else {
					output.push(*option);
				}
			}

			Some(Change::swap([
				BrainIr::output_cells(output),
				BrainIr::change_cell_at(a.value().wrapping_add(b.value()), a.offset()),
			]))
		}
		[
			BrainIr::MovePointer(x),
			BrainIr::Output(OutputOptions::Cells(options)),
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			BrainIr::output_cells(options.iter().map(|option| {
				ChangeCellOptions::new(option.value(), option.offset().wrapping_add(*x))
			})),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			l,
		] if l.is_zeroing_cell() && !change_options.is_offset() => {
			let mut output = Vec::with_capacity(output_options.len());

			for option in output_options {
				if option.is_offset() {
					output.push(*option);
				} else {
					output.push(ChangeCellOptions::new(
						option.value().wrapping_add(change_options.value()),
						0,
					));
				}
			}

			Some(Change::swap([BrainIr::output_cells(output), l.clone()]))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::MovePointer(x),
			BrainIr::Output(OutputOptions::Cells(output_options)),
		] => {
			let range = set_many_options.range();

			if !range.contains(x) {
				return None;
			}

			let range = (range.start.wrapping_sub(*x))..(range.end.wrapping_sub(*x));

			if output_options.iter().any(|x| !range.contains(&x.offset())) {
				return None;
			}

			let new_set_many_options =
				SetManyCellsOptions::new(set_many_options.values().iter().copied(), range.start);

			let mut chars = Vec::with_capacity(output_options.len());

			for option in output_options {
				let char = new_set_many_options
					.value_at(option.offset())?
					.wrapping_add_signed(option.value());

				chars.push(char);
			}

			Some(Change::swap([
				BrainIr::output_str(chars),
				BrainIr::set_many_cells(
					set_many_options.values().iter().copied(),
					set_many_options.start(),
				),
				BrainIr::move_pointer(*x),
			]))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::MovePointer(x),
			BrainIr::Output(OutputOptions::Cell(output_options)),
		] => {
			let range = set_many_options.range();

			if !range.contains(x) {
				return None;
			}

			let range = (range.start.wrapping_sub(*x))..(range.end.wrapping_sub(*x));

			if !range.contains(&output_options.offset()) {
				return None;
			}

			let new_set_many_options =
				SetManyCellsOptions::new(set_many_options.values().iter().copied(), range.start);

			let char = new_set_many_options
				.value_at(output_options.offset())?
				.wrapping_add_signed(output_options.value());

			Some(Change::swap([
				BrainIr::output_char(char),
				BrainIr::set_many_cells(
					set_many_options.values().iter().copied(),
					set_many_options.start(),
				),
				BrainIr::move_pointer(*x),
			]))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			BrainIr::Boundary,
		] => {
			let mut new_offsets = Vec::with_capacity(output_options.len());

			for option in output_options {
				if option.offset() == change_options.offset() {
					new_offsets.push(ChangeCellOptions::new(
						option.value().wrapping_add(change_options.value()),
						option.offset(),
					));
				} else {
					new_offsets.push(*option);
				}
			}

			Some(Change::swap([
				BrainIr::output_cells(new_offsets),
				BrainIr::boundary(),
			]))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::Boundary,
		] if change_options.offset() == output_options.offset() => Some(Change::swap([
			BrainIr::output_offset_cell_at(
				output_options.value().wrapping_add(change_options.value()),
				output_options.offset(),
			),
			BrainIr::boundary(),
		])),
		[
			BrainIr::MovePointer(offset),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::Boundary,
		] => Some(Change::swap([
			BrainIr::output_offset_cell_at(
				output_options.value(),
				offset.wrapping_add(output_options.offset()),
			),
			BrainIr::boundary(),
		])),
		[
			BrainIr::MovePointer(offset),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			BrainIr::Boundary,
		] => {
			Some(Change::swap([
				BrainIr::output_cells(output_options.iter().copied().map(|opt| {
					ChangeCellOptions::new(opt.value(), offset.wrapping_add(opt.offset()))
				})),
				BrainIr::boundary(),
			]))
		}
		[
			BrainIr::MovePointer(..),
			BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			BrainIr::Boundary,
		] => Some(Change::remove_offset(0)),
		[
			l,
			BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			BrainIr::Boundary,
		] if !l.has_io() => Some(Change::remove_offset(0)),
		_ => None,
	}
}

pub fn optimize_changes_and_writes(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(a),
			BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			BrainIr::SetCell(b),
		] if !a.is_offset() && !b.is_offset() => Some(Change::remove_offset(0)),
		[
			first
			@ (BrainIr::SetCell(..) | BrainIr::SetManyCells { .. } | BrainIr::SetRange { .. }),
			out @ BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			second @ (BrainIr::SetCell(..)
			| BrainIr::SetManyCells { .. }
			| BrainIr::SetRange { .. }),
		] => Some(Change::swap([out.clone(), first.clone(), second.clone()])),
		_ => None,
	}
}

pub fn optimize_constant_shifts(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(set_options),
			BrainIr::FetchValueFrom(fetch_options),
		] if set_options.offset() == fetch_options.offset() => Some(Change::swap([
			BrainIr::clear_cell_at(set_options.offset()),
			BrainIr::set_cell(set_options.value().wrapping_mul(fetch_options.factor())),
		])),
		[
			BrainIr::SetCell(set_options),
			BrainIr::TakeValueTo(take_options),
		] if !set_options.is_offset() => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::move_pointer(take_options.offset()),
			BrainIr::change_cell(set_options.value().wrapping_mul(take_options.factor()) as i8),
		])),
		[
			BrainIr::SetCell(set_options),
			BrainIr::MoveValueTo(move_options),
		] if !set_options.is_offset() => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::change_cell_at(
				set_options.value().wrapping_mul(move_options.factor()) as i8,
				move_options.offset(),
			),
		])),
		[
			BrainIr::MoveValueTo(move_options),
			BrainIr::SetCell(set_options),
		] if set_options.is_offset() && move_options.offset() == set_options.offset() => {
			Some(Change::swap([
				BrainIr::clear_cell(),
				BrainIr::set_cell_at(set_options.value(), set_options.offset()),
			]))
		}
		_ => None,
	}
}

pub fn optimize_sub_cell_from(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SubCell(SubOptions::CellAt(options)),
			BrainIr::MovePointer(y),
		] if options.offset() == *y => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::sub_from_cell(options.factor(), -y),
		])),
		_ => None,
	}
}

pub fn optimize_sub_cell_from_with_set(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SubCell(SubOptions::CellAt(sub_options)),
			BrainIr::SetCell(set_options),
			BrainIr::MovePointer(y),
		] if sub_options.offset() == *y && !set_options.is_offset() => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::sub_from_cell(sub_options.factor(), -y),
			BrainIr::set_cell_at(set_options.value(), -y),
		])),
		_ => None,
	}
}

pub fn remove_redundant_shifts(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::TakeValueTo(take_options),
			BrainIr::SetCell(set_options),
		] if !set_options.is_offset() => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::move_pointer(take_options.offset()),
			BrainIr::set_cell(set_options.value()),
		])),
		[
			BrainIr::MoveValueTo(move_options),
			BrainIr::SetCell(set_options),
		] if move_options.offset() == set_options.offset() && set_options.is_offset() => {
			Some(Change::swap([
				BrainIr::clear_cell(),
				BrainIr::set_cell_at(set_options.value(), set_options.offset()),
			]))
		}
		[
			BrainIr::MoveValueTo(move_options),
			BrainIr::ReplaceValueFrom(replace_options),
		] if move_options.offset() == replace_options.offset()
			&& matches!(move_options.factor(), 1)
			&& matches!(replace_options.factor(), 1) =>
		{
			Some(Change::replace(BrainIr::fetch_value_from(
				1,
				move_options.offset(),
			)))
		}
		_ => None,
	}
}

pub fn optimize_mem_sets(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(a), BrainIr::SetCell(b)] if a.value() == b.value() => {
			let x = a.offset();
			let y = b.offset();

			let min = cmp::min(x, y);
			let max = cmp::max(x, y);

			if !matches!((max - min).unsigned_abs(), 1) {
				return None;
			}

			Some(Change::replace(BrainIr::set_range(a.value(), min, max)))
		}
		[
			BrainIr::SetCell(set_options),
			BrainIr::SetRange(set_range_options),
		]
		| [
			BrainIr::SetRange(set_range_options),
			BrainIr::SetCell(set_options),
		] => {
			let x = set_options.offset();
			let range = set_range_options.range();
			let start = *range.start();
			let end = *range.end();

			if !matches!((x - start).unsigned_abs(), 1) && !matches!((end - x).unsigned_abs(), 1) {
				return None;
			}

			let min = cmp::min(x, start);

			if set_range_options.value() == set_options.value() {
				let max = cmp::max(x, end);

				Some(Change::replace(BrainIr::set_range(
					set_range_options.value(),
					min,
					max,
				)))
			} else {
				let mut values = range
					.clone()
					.map(|_| set_range_options.value())
					.collect::<Vec<_>>();

				let new_offset_raw = x.wrapping_add(min.abs());

				assert!((0..=i32::MAX).contains(&new_offset_raw));

				let new_offset = new_offset_raw as usize;

				if range.contains(&x) {
					if new_offset >= values.len() {
						values.push(set_options.value());
					} else {
						values[new_offset] = set_options.value();
					}
				} else {
					values.insert_or_push(new_offset, set_options.value());
				}

				Some(Change::replace(BrainIr::set_many_cells(values, min)))
			}
		}
		[BrainIr::SetRange(a), BrainIr::SetRange(b)]
			if a.end().wrapping_add(1) == b.start() && a.value() == b.value() =>
		{
			Some(Change::replace(BrainIr::set_range(
				a.value(),
				a.start(),
				b.end(),
			)))
		}
		[BrainIr::SetRange(a), BrainIr::SetRange(b)]
			if b.end().wrapping_add(1) == a.start() && a.value() == b.value() =>
		{
			Some(Change::replace(BrainIr::set_range(
				a.value(),
				b.start(),
				a.end(),
			)))
		}
		[BrainIr::SetRange(a), BrainIr::SetRange(b)] if a.range() == b.range() => {
			Some(Change::remove_offset(0))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::SetRange(set_range_options),
		] if set_range_options.range().contains(&change_options.offset()) => {
			Some(Change::remove_offset(0))
		}
		[BrainIr::SetCell(a), BrainIr::SetCell(b)] => {
			let x = a.offset();
			let y = b.offset();
			let min = cmp::min(x, y);
			let max = cmp::max(x, y);

			if !matches!((max - min).unsigned_abs(), 1) {
				return None;
			}

			let (a, b) = if x == min {
				(a.value(), b.value())
			} else {
				(b.value(), a.value())
			};

			Some(Change::replace(BrainIr::set_many_cells([a, b], min)))
		}
		[
			BrainIr::SetCell(set_options),
			BrainIr::SetManyCells(set_many_options),
		]
		| [
			BrainIr::SetManyCells(set_many_options),
			BrainIr::SetCell(set_options),
		] if set_many_options.range().contains(&set_options.offset()) => {
			let mut set_many_options = set_many_options.clone();

			if !set_many_options.set_value_at(set_options.offset(), set_options.value()) {
				return None;
			}

			Some(Change::replace(set_many_options.convert::<BrainIr>()))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::SetCell(set_options),
		]
		| [
			BrainIr::SetCell(set_options),
			BrainIr::SetManyCells(set_many_options),
		] => {
			let x = set_options.offset();
			let range = set_many_options.range();

			if x != range.end {
				return None;
			}

			Some(Change::replace(BrainIr::set_many_cells(
				set_many_options
					.values()
					.iter()
					.copied()
					.chain(iter::once(set_options.value())),
				range.start,
			)))
		}
		[BrainIr::SetManyCells(a), BrainIr::SetManyCells(b)]
			if a.start() == b.start() && a.values().len() <= b.values().len() =>
		{
			Some(Change::remove_offset(0))
		}
		[BrainIr::SetManyCells(a), BrainIr::SetManyCells(b)]
			if a.range().end == b.range().start =>
		{
			Some(Change::replace(BrainIr::set_many_cells(
				a.values().iter().copied().chain(b.values().iter().copied()),
				a.start().get_or_zero(),
			)))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::SetRange(set_range_options),
		] if set_many_options.range().end == *set_range_options.range().start() => {
			let mut new_values = set_many_options.values().to_owned();

			for _ in set_range_options.range() {
				new_values.push(set_range_options.value());
			}

			Some(Change::replace(BrainIr::set_many_cells(
				new_values,
				set_many_options.start(),
			)))
		}
		[
			BrainIr::SetRange(set_range_options),
			BrainIr::SetManyCells(set_many_options),
		] => {
			let set_many_count = set_many_options.range().len();
			let set_range_count = set_range_options.range().count();

			if set_many_options.start() == set_range_options.start()
				&& set_many_count >= set_range_count
			{
				Some(Change::remove_offset(0))
			} else {
				None
			}
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::MovePointer(x),
		] if set_many_options.start() == *x => Some(Change::swap([
			BrainIr::move_pointer(*x),
			BrainIr::set_many_cells(set_many_options.values().iter().copied(), 0),
		])),
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::FetchValueFrom(fetch_options),
		] if set_many_options.range().contains(&fetch_options.offset()) => {
			let fetched_value = set_many_options.value_at(fetch_options.offset())?;

			let current_cell = set_many_options.value_at(0)?;

			let mut set_many_options = set_many_options.clone();

			if !set_many_options.set_value_at(fetch_options.offset(), 0) {
				return None;
			}

			let scaled_fetched_value = fetched_value.wrapping_mul(fetch_options.factor());

			let added_value = current_cell.wrapping_add(scaled_fetched_value);

			if !set_many_options.set_value_at(0, added_value) {
				return None;
			}

			Some(Change::replace(set_many_options.convert::<BrainIr>()))
		}
		_ => None,
	}
}

pub fn optimize_mem_set_move_change(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::MovePointer(x),
			BrainIr::ChangeCell(change_options),
		] if !change_options.is_offset() => {
			let mut range = set_many_options.range();

			if !range.contains(x) {
				return None;
			}

			let cell_index = range.position(|y| y == *x)?;

			let mut values = set_many_options.values().to_owned();

			values[cell_index] = values[cell_index].wrapping_add_signed(change_options.value());

			Some(Change::swap([
				BrainIr::set_many_cells(values, set_many_options.start()),
				BrainIr::move_pointer(*x),
			]))
		}
		[
			BrainIr::SetRange(set_range_options),
			BrainIr::MovePointer(x),
			BrainIr::ChangeCell(change_options),
		] if !change_options.is_offset() => {
			let mut set_many_options = SetManyCellsOptions::from(*set_range_options);

			if !set_many_options.set_value_at(*x, change_options.value() as u8) {
				return None;
			}

			Some(Change::swap([
				BrainIr::SetManyCells(set_many_options),
				BrainIr::move_pointer(*x),
			]))
		}
		_ => None,
	}
}

pub fn optimize_if_nz_when_zeroing(ops: [&BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::IfNotZero(ops) | BrainIr::DynamicLoop(ops)] => match &**ops {
			[x] if x.needs_nonzero_cell() && x.is_zeroing_cell() => {
				Some(Change::replace(x.clone()))
			}
			_ => None,
		},
		_ => None,
	}
}

pub fn unroll_constant_duplicate_cell(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(set_options),
			BrainIr::DuplicateCell { values },
		] if !set_options.is_offset() => {
			let mut output = Vec::with_capacity(values.len() + 1);

			output.push(BrainIr::clear_cell());

			for option in values {
				let factored_value = option.factor().wrapping_mul(set_options.value() as i8);

				output.push(BrainIr::change_cell_at(factored_value, option.offset()));
			}

			Some(Change::swap(output))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::DuplicateCell { values },
		] if values
			.iter()
			.all(|x| set_many_options.value_at(x.offset()).is_some()) =>
		{
			let current_cell_value = set_many_options.value_at(0)?;

			let mut set_many_options = set_many_options.clone();

			if !set_many_options.set_value_at(0, 0) {
				return None;
			}

			for dupe_option in values.iter().copied() {
				let new_value_to_set = current_cell_value.wrapping_mul(dupe_option.factor() as u8);

				set_many_options.set_value_at(dupe_option.offset(), new_value_to_set);
			}

			Some(Change::replace(BrainIr::SetManyCells(set_many_options)))
		}
		_ => None,
	}
}

pub fn unroll_constant_if_nz(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(set_options), BrainIr::IfNotZero(ops)]
			if matches!(set_options.into_parts(), (1..=u8::MAX, 0)) =>
		{
			Some(Change::swap(
				iter::once(BrainIr::set_cell(set_options.value())).chain(ops.iter().cloned()),
			))
		}
		_ => None,
	}
}

pub fn unroll_basic_dynamic_loop(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(set_options), l @ BrainIr::DynamicLoop(ops)]
			if matches!(set_options.into_parts(), (1..=u8::MAX, 0))
				&& matches!(calculate_ptr_movement(ops)?, 0)
				&& matches!(ops.as_slice(), [.., BrainIr::ChangeCell(change_options)] if matches!(change_options.into_parts(), (i8::MIN..0, 0)))
				&& !l.loop_has_movement()? =>
		{
			if ops
				.iter()
				.any(|op| matches!(op, BrainIr::DynamicLoop(..) | BrainIr::IfNotZero(..)))
			{
				return None;
			}

			let (without_decrement, decrement) = {
				let mut owned = ops.clone();
				let decrement = owned.pop()?;

				let BrainIr::ChangeCell(change_cell_options) = decrement else {
					return None;
				};

				(owned, change_cell_options.value())
			};

			let mut out =
				Vec::with_capacity(without_decrement.len() * set_options.value() as usize);

			for _ in (0..set_options.value()).step_by(decrement.unsigned_abs() as usize) {
				out.extend_from_slice(&without_decrement);
			}

			Some(Change::swap(out))
		}
		[BrainIr::SetCell(set_options), l @ BrainIr::DynamicLoop(ops)]
			if matches!(set_options.into_parts(), (1..=u8::MAX, 0))
				&& matches!(calculate_ptr_movement(ops)?, 0)
				&& matches!(ops.as_slice(), [BrainIr::ChangeCell(change_options), ..] if matches!(change_options.into_parts(), (i8::MIN..0, 0)))
				&& !l.loop_has_movement()? =>
		{
			if ops
				.iter()
				.any(|op| matches!(op, BrainIr::DynamicLoop(..) | BrainIr::IfNotZero(..)))
			{
				return None;
			}

			let (without_decrement, decrement) = {
				let mut owned = ops.clone();
				let decrement = owned.remove(0);

				let BrainIr::ChangeCell(change_options) = decrement else {
					return None;
				};

				(owned, change_options.value())
			};

			let mut out =
				Vec::with_capacity(without_decrement.len() * set_options.value() as usize);

			for _ in (0..set_options.value()).step_by(decrement.unsigned_abs() as usize) {
				out.extend_from_slice(&without_decrement);
			}

			Some(Change::swap(out))
		}
		_ => None,
	}
}

pub fn unroll_if_nz(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			set @ BrainIr::SetManyCells(set_many_options),
			BrainIr::IfNotZero(ops),
		] if !matches!(set_many_options.value_at(0)?, 0) => Some(Change::swap(
			iter::once(set.clone()).chain(ops.iter().cloned()),
		)),
		_ => None,
	}
}

pub fn optimize_scale_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::TakeValueTo(first_take_options),
			BrainIr::TakeValueTo(second_take_options),
		] if matches!(
			first_take_options
				.offset()
				.wrapping_add(second_take_options.offset()),
			0
		) =>
		{
			Some(Change::swap([
				BrainIr::scale_value(first_take_options.factor()),
				BrainIr::fetch_value_from(1, first_take_options.offset()),
				BrainIr::scale_value(second_take_options.factor()),
			]))
		}
		[
			BrainIr::TakeValueTo(take_options),
			BrainIr::MoveValueTo(move_options),
		] if matches!(take_options.offset().wrapping_add(move_options.offset()), 0) => {
			Some(Change::swap([
				BrainIr::scale_value(take_options.factor()),
				BrainIr::fetch_value_from(1, take_options.offset()),
				BrainIr::scale_value(move_options.factor()),
				BrainIr::move_pointer(take_options.offset()),
			]))
		}
		[BrainIr::ScaleValue(a), BrainIr::ScaleValue(b)] => {
			Some(Change::replace(BrainIr::scale_value(a.wrapping_mul(*b))))
		}
		[
			BrainIr::ScaleValue(factor),
			BrainIr::TakeValueTo(take_options),
		] if matches!(take_options.factor(), 2..=u8::MAX) => Some(Change::swap([
			BrainIr::scale_value(factor.wrapping_mul(take_options.factor())),
			BrainIr::take_value_to(1, take_options.offset()),
		])),
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::ScaleValue(factor),
		] => {
			let value = set_many_options.value_at(0)?;

			let mut set_many_options = set_many_options.clone();

			let new_value = value.wrapping_mul(*factor);

			if !set_many_options.set_value_at(0, new_value) {
				return None;
			}

			Some(Change::replace(BrainIr::SetManyCells(set_many_options)))
		}
		_ => None,
	}
}
