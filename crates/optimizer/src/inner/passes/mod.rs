#![allow(clippy::trivially_copy_pass_by_ref)]

mod loops;
mod sort;

use std::{cmp, iter};

use frick_ir::{BrainIr, CellChangeOptions, OutputOptions, SubType, is_range};
use frick_utils::{GetOrZero as _, InsertOrPush as _};

pub use self::{loops::*, sort::*};
use super::Change;

pub fn optimize_consecutive_instructions(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(a, x), BrainIr::ChangeCell(b, y)] if *x == *y => Some(if *a == -b {
			Change::remove()
		} else {
			Change::replace(BrainIr::change_cell_at(a.wrapping_add(*b), x.get_or_zero()))
		}),
		[BrainIr::MovePointer(a), BrainIr::MovePointer(b)] => Some(if *a == -b {
			Change::remove()
		} else {
			Change::replace(BrainIr::move_pointer(a.wrapping_add(*b)))
		}),
		_ => None,
	}
}

pub fn optimize_sets(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(.., a) | BrainIr::ChangeCell(.., a),
			BrainIr::SetCell(.., b),
		] if *a == *b => Some(Change::remove_offset(0)),
		[BrainIr::SetCell(i1, x), BrainIr::ChangeCell(i2, y)] if *x == *y => Some(Change::replace(
			BrainIr::set_cell_at(i1.wrapping_add_signed(*i2), x.get_or_zero()),
		)),
		[l, BrainIr::ChangeCell(i1, None)] if l.is_zeroing_cell() => {
			Some(Change::swap([l.clone(), BrainIr::set_cell(*i1 as u8)]))
		}
		[BrainIr::SetCell(.., None), BrainIr::InputIntoCell] => Some(Change::remove_offset(0)),
		[l, BrainIr::SetCell(0, None)] if l.is_zeroing_cell() => Some(Change::remove_offset(1)),
		[BrainIr::SetCell(.., x), BrainIr::SetRange(options)] => {
			let range = options.range();

			if range.contains(&x.get_or_zero()) {
				Some(Change::remove_offset(0))
			} else {
				None
			}
		}
		[
			value_from @ (BrainIr::ReplaceValueFrom(options) | BrainIr::FetchValueFrom(options)),
			BrainIr::ChangeCell(a, x),
		] if options.offset() == x.get_or_zero() => Some(Change::swap([
			value_from.clone(),
			BrainIr::set_cell_at(*a as u8, x.get_or_zero()),
		])),
		[BrainIr::TakeValueTo(options), BrainIr::ChangeCell(a, x)]
			if options.offset() == -x.get_or_zero() =>
		{
			Some(Change::swap([
				BrainIr::TakeValueTo(*options),
				BrainIr::set_cell_at(*a as u8, x.get_or_zero()),
			]))
		}
		_ => None,
	}
}

pub const fn remove_noop_instructions(ops: [&BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(0, ..) | BrainIr::MovePointer(0)] => Some(Change::remove()),
		_ => None,
	}
}

pub fn fix_boundary_instructions(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::Boundary, BrainIr::ChangeCell(a, x)] => Some(Change::swap([
			BrainIr::boundary(),
			BrainIr::set_cell_at(*a as u8, x.get_or_zero()),
		])),
		[l, BrainIr::Boundary] if !l.has_output() => Some(Change::remove_offset(0)),
		[BrainIr::Boundary, BrainIr::Output(OutputOptions::Cell(..))] => Some(Change::swap([
			BrainIr::boundary(),
			BrainIr::output_char(0),
			BrainIr::set_cell(0),
		])),
		_ => None,
	}
}

pub fn optimize_initial_sets(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::Boundary,
			set @ BrainIr::SetCell(.., x),
			BrainIr::ChangeCell(b, y),
		] if *x != *y => Some(Change::swap([
			BrainIr::boundary(),
			set.clone(),
			BrainIr::set_cell_at(*b as u8, y.get_or_zero()),
		])),
		[BrainIr::Boundary, BrainIr::SetCell(0, ..), ..] => Some(Change::remove_offset(1)),
		[BrainIr::Boundary, .., BrainIr::SetCell(0, ..)] => Some(Change::remove_offset(2)),
		_ => None,
	}
}

pub fn add_offsets(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::MovePointer(x),
			BrainIr::ChangeCell(i, y),
			BrainIr::MovePointer(z),
		] => Some(Change::swap([
			BrainIr::change_cell_at(*i, x.wrapping_add(y.get_or_zero())),
			BrainIr::move_pointer(x.wrapping_add(*z)),
		])),
		[
			BrainIr::MovePointer(x),
			BrainIr::SetCell(i, y),
			BrainIr::MovePointer(z),
		] => Some(Change::swap([
			BrainIr::set_cell_at(*i, x.wrapping_add(y.get_or_zero())),
			BrainIr::move_pointer(x.wrapping_add(*z)),
		])),
		[
			BrainIr::MovePointer(x),
			BrainIr::SetRange(options),
			BrainIr::MovePointer(y),
		] => {
			let start = options.start.wrapping_add(*x);
			let end = options.end.wrapping_add(*x);

			let set_range_instr = BrainIr::set_range(options.value, start, end);

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
		[BrainIr::SetCell(a, Some(x)), BrainIr::MovePointer(y)] if x.get() == *y => {
			Some(Change::swap([
				BrainIr::move_pointer(*y),
				BrainIr::set_cell(*a),
			]))
		}
		[BrainIr::ChangeCell(a, Some(x)), BrainIr::MovePointer(y)] if x.get() == *y => {
			Some(Change::swap([
				BrainIr::move_pointer(*y),
				BrainIr::change_cell(*a),
			]))
		}
		[
			BrainIr::Output(OutputOptions::Cell(options)),
			BrainIr::MovePointer(y),
		] if options.offset() == *y => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::output_offset_cell(options.value()),
		])),
		[
			BrainIr::Output(OutputOptions::Cells(options)),
			BrainIr::MovePointer(y),
		] if options.iter().all(|x| x.offset() == *y) => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::output_cells(
				options
					.iter()
					.copied()
					.map(|x| CellChangeOptions::new(x.value(), 0)),
			),
		])),
		_ => None,
	}
}

pub fn optimize_move_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::TakeValueTo(options), BrainIr::MovePointer(y)] => Some(Change::swap([
			BrainIr::move_value_to(options.value(), options.offset()),
			BrainIr::move_pointer(options.offset().wrapping_add(*y)),
		])),
		_ => None,
	}
}

pub fn optimize_move_value_from_duplicate_cells(ops: [&BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::DuplicateCell { values }] if matches!(values.len(), 1) => {
			let data = values.first().copied()?;

			let value = data.value();
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
			Change::replace(BrainIr::take_value_to(options.value(), options.offset())),
		),
		_ => None,
	}
}

pub fn optimize_fetch_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::MovePointer(x), BrainIr::TakeValueTo(options)] => Some(Change::swap([
			BrainIr::move_pointer(x.wrapping_add(options.offset())),
			BrainIr::fetch_value_from(options.value(), -options.offset()),
		])),
		[BrainIr::MovePointer(x), BrainIr::MoveValueTo(options)] if *x == -options.offset() => {
			Some(Change::swap([
				BrainIr::fetch_value_from(options.value(), *x),
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
				.any(|v| v.offset() == options.offset() && matches!(v.value(), 1))
			{
				return None;
			}

			if !matches!(values.len(), 2) {
				return None;
			}

			let other_move_options = {
				let offset = values
					.iter()
					.position(|x| x.offset() != options.offset() && x.value().is_positive())?;

				values.get(offset).copied()?
			};

			Some(Change::swap([
				BrainIr::copy_value_to(
					other_move_options.value() as u8,
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
			BrainIr::SetCell(value, x),
			BrainIr::Output(OutputOptions::Cell(options)),
		] if options.offset() == x.get_or_zero() => Some(Change::swap([
			BrainIr::output_char(value.wrapping_add_signed(options.value())),
			BrainIr::set_cell_at(*value, x.get_or_zero()),
		])),
		[l, BrainIr::Output(OutputOptions::Cell(options))]
			if l.is_zeroing_cell() && matches!(options.offset(), 0) =>
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
			BrainIr::SetCell(value, None),
			BrainIr::Output(OutputOptions::Cells(options)),
		] if options.iter().all(|x| matches!(x.offset(), 0)) => {
			let mut chars = Vec::with_capacity(options.len());

			for value_offset in options.iter().map(|x| x.value()) {
				chars.push(value.wrapping_add_signed(value_offset));
			}

			Some(Change::swap([
				BrainIr::output_str(chars),
				BrainIr::set_cell(*value),
			]))
		}
		[
			BrainIr::Output(OutputOptions::Cells(a)),
			BrainIr::Output(OutputOptions::Cells(b)),
		] => Some(Change::replace(BrainIr::output_cells(
			a.iter().copied().chain(b.iter().copied()),
		))),
		[
			BrainIr::SetCell(a, x),
			BrainIr::Output(OutputOptions::Cells(options)),
		] if options
			.iter()
			.all(|option| option.offset() != x.get_or_zero()) =>
		{
			Some(Change::swap([
				BrainIr::output_cells(options.iter().copied()),
				BrainIr::set_cell_at(*a, x.get_or_zero()),
			]))
		}
		[
			BrainIr::SetManyCells(set_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
		] => {
			let char_at = set_options.value_at(output_options.offset())?;

			Some(Change::swap([
				BrainIr::output_char(char_at),
				BrainIr::set_many_cells(
					set_options.values.iter().copied(),
					set_options.start.get_or_zero(),
				),
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

			let mut str = Vec::new();

			for opt in output_options {
				let current_value = set_options.value_at(opt.offset())?;

				str.push(current_value);
			}

			Some(Change::swap([
				BrainIr::output_str(str),
				BrainIr::set_many_cells(
					set_options.values.iter().copied(),
					set_options.start.get_or_zero(),
				),
			]))
		}
		_ => None,
	}
}

pub fn optimize_offset_writes(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(a, None),
			BrainIr::Output(OutputOptions::Cell(options)),
			BrainIr::ChangeCell(b, None),
		] => {
			if *a == -b {
				Some(Change::replace(BrainIr::output_offset_cell(
					a.wrapping_add(options.value()),
				)))
			} else if options.is_default() {
				Some(Change::swap([
					BrainIr::output_offset_cell(*a),
					BrainIr::change_cell(a.wrapping_add(*b)),
				]))
			} else if matches!(options.offset(), 0) {
				Some(Change::swap([
					BrainIr::change_cell(a.wrapping_add(*b)),
					BrainIr::output_offset_cell(options.value().wrapping_sub(*b)),
				]))
			} else {
				None
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
		#[allow(clippy::suspicious_operation_groupings)]
		[
			BrainIr::ChangeCell(a, Some(x)),
			BrainIr::Output(OutputOptions::Cell(options)),
			BrainIr::ChangeCell(b, Some(y)),
		] if x.get_or_zero() == options.offset() && *x == *y => Some(Change::swap([
			BrainIr::output_offset_cell_at(options.value().wrapping_add(*a), options.offset()),
			BrainIr::change_cell_at(a.wrapping_add(*b), x.get_or_zero()),
		])),
		[
			BrainIr::ChangeCell(a, None),
			BrainIr::Output(OutputOptions::Cell(options)),
			BrainIr::SetCell(b, None),
		] if matches!(options.offset(), 0) => Some(Change::swap([
			BrainIr::output_offset_cell(a.wrapping_add(options.value())),
			BrainIr::set_cell(*b),
		])),
		[
			BrainIr::ChangeCell(a, x),
			BrainIr::Output(OutputOptions::Cells(options)),
			BrainIr::ChangeCell(b, y),
		] => {
			let x = x.get_or_zero();
			let y = y.get_or_zero();

			if x != -y {
				return None;
			}

			let mut output = Vec::with_capacity(options.len());

			for option in options {
				if option.offset() == x {
					output.push(CellChangeOptions::new(option.value().wrapping_add(*a), x));
				} else {
					output.push(*option);
				}
			}

			Some(Change::swap([
				BrainIr::output_cells(output),
				BrainIr::change_cell_at(a.wrapping_add(*b), x),
			]))
		}
		[
			BrainIr::MovePointer(x),
			BrainIr::Output(OutputOptions::Cells(options)),
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			BrainIr::output_cells(options.iter().map(|option| {
				CellChangeOptions::new(option.value(), option.offset().wrapping_add(*x))
			})),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::ChangeCell(x, None),
			BrainIr::Output(OutputOptions::Cells(options)),
			l,
		] if l.is_zeroing_cell() => {
			let mut output = Vec::with_capacity(options.len());

			for option in options {
				if matches!(option.offset(), 0) {
					output.push(CellChangeOptions::new(option.value().wrapping_add(*x), 0));
				} else {
					output.push(*option);
				}
			}

			Some(Change::swap([BrainIr::output_cells(output), l.clone()]))
		}
		_ => None,
	}
}

pub fn optimize_changes_and_writes(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(.., None),
			BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			BrainIr::SetCell(.., None),
		] => Some(Change::remove_offset(0)),
		[
			first
			@ (BrainIr::SetCell(..) | BrainIr::SetManyCells { .. } | BrainIr::SetRange { .. }),
			out @ BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			second @ (BrainIr::SetCell(..)
			| BrainIr::SetManyCells { .. }
			| BrainIr::SetRange { .. }),
		] => Some(Change::swap([out.clone(), first.clone(), second.clone()])),
		[
			BrainIr::ChangeCell(value, None),
			BrainIr::Output(OutputOptions::Cells(options)),
			BrainIr::SetCell(value_to_set, None),
		] if options.iter().all(|x| matches!(x.offset(), 0)) => Some(Change::swap([
			BrainIr::output_cells(
				options
					.iter()
					.copied()
					.map(|x| CellChangeOptions::new(x.value().wrapping_add(*value), 0)),
			),
			BrainIr::set_cell(*value_to_set),
		])),
		_ => None,
	}
}

pub fn optimize_constant_shifts(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(a, x), BrainIr::FetchValueFrom(options)]
			if x.get_or_zero() == options.offset() =>
		{
			Some(Change::swap([
				BrainIr::clear_cell_at(x.get_or_zero()),
				BrainIr::set_cell(a.wrapping_mul(options.value())),
			]))
		}
		[BrainIr::SetCell(a, None), BrainIr::TakeValueTo(options)] => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::move_pointer(options.offset()),
			BrainIr::change_cell(a.wrapping_mul(options.value()) as i8),
		])),
		[BrainIr::SetCell(a, None), BrainIr::MoveValueTo(options)] => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::change_cell_at(a.wrapping_mul(options.value()) as i8, options.offset()),
		])),
		[BrainIr::MoveValueTo(.., x), BrainIr::SetCell(a, Some(y))] if x.offset() == y.get() => {
			Some(Change::swap([
				BrainIr::clear_cell(),
				BrainIr::set_cell_at(*a, y.get()),
			]))
		}
		[BrainIr::SetCell(a, x), BrainIr::ReplaceValueFrom(options)]
			if x.get_or_zero() == options.offset() =>
		{
			Some(Change::swap([
				BrainIr::clear_cell_at(x.get_or_zero()),
				BrainIr::set_cell(a.wrapping_mul(options.value())),
			]))
		}
		_ => None,
	}
}

pub fn optimize_sub_cell_from(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SubCell(SubType::CellAt(options)),
			BrainIr::MovePointer(y),
		] if options.offset() == *y => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::sub_from_cell(options.value(), -y),
		])),
		_ => None,
	}
}

pub fn optimize_sub_cell_from_with_set(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SubCell(SubType::CellAt(options)),
			BrainIr::SetCell(a, None),
			BrainIr::MovePointer(y),
		] if options.offset() == *y => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::sub_from_cell(options.value(), -y),
			BrainIr::set_cell_at(*a, -y),
		])),
		_ => None,
	}
}

pub fn optimize_constant_sub(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(a, None),
			BrainIr::SubCell(SubType::CellAt(options)),
		] => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::move_pointer(options.offset()),
			BrainIr::SubCell(SubType::Value(*a)),
			BrainIr::move_pointer(-options.offset()),
		])),
		_ => None,
	}
}

pub fn remove_redundant_shifts(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::TakeValueTo(options), BrainIr::SetCell(value, None)] => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::move_pointer(options.offset()),
			BrainIr::set_cell(*value),
		])),
		[
			BrainIr::MoveValueTo(options),
			BrainIr::SetCell(value, Some(offset)),
		] if options.offset() == offset.get() => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::set_cell_at(*value, offset.get()),
		])),
		[
			BrainIr::FetchValueFrom(options) | BrainIr::ReplaceValueFrom(options),
			BrainIr::SetCell(value, None),
		] => Some(Change::swap([
			BrainIr::clear_cell_at(options.offset()),
			BrainIr::set_cell(*value),
		])),
		[
			BrainIr::MoveValueTo(move_options),
			BrainIr::ReplaceValueFrom(replace_options),
		] if move_options.offset() == replace_options.offset()
			&& matches!(move_options.value(), 1)
			&& matches!(replace_options.value(), 1) =>
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
		[BrainIr::SetCell(a, x), BrainIr::SetCell(b, y)] if *a == *b => {
			let x = x.get_or_zero();
			let y = y.get_or_zero();
			let min = cmp::min(x, y);
			let max = cmp::max(x, y);

			if !matches!((max - min).unsigned_abs(), 1) {
				return None;
			}

			Some(Change::replace(BrainIr::set_range(*a, min, max)))
		}
		[BrainIr::SetRange(options), BrainIr::SetCell(b, x)]
		| [BrainIr::SetCell(b, x), BrainIr::SetRange(options)] => {
			let x = x.get_or_zero();
			let range = options.range();
			let start = *range.start();
			let end = *range.end();

			if !matches!((x - start).unsigned_abs(), 1) && !matches!((end - x).unsigned_abs(), 1) {
				return None;
			}

			let min = cmp::min(x, start);

			if options.value == *b {
				let max = cmp::max(x, end);

				Some(Change::replace(BrainIr::set_range(options.value, min, max)))
			} else {
				let mut values = range.clone().map(|_| options.value).collect::<Vec<_>>();

				let new_offset_raw = x.wrapping_add(min.abs());

				assert!((0..=i32::MAX).contains(&new_offset_raw));

				let new_offset = new_offset_raw as usize;

				if range.contains(&x) {
					if new_offset >= values.len() {
						values.push(*b);
					} else {
						values[new_offset] = *b;
					}
				} else {
					values.insert_or_push(new_offset, *b);
				}

				Some(Change::replace(BrainIr::set_many_cells(values, min)))
			}
		}
		[BrainIr::SetRange(a), BrainIr::SetRange(b)]
			if a.end.wrapping_add(1) == b.start && a.value == b.value =>
		{
			Some(Change::replace(BrainIr::set_range(a.value, a.start, b.end)))
		}
		[BrainIr::SetRange(a), BrainIr::SetRange(b)]
			if b.end.wrapping_add(1) == a.start && a.value == b.value =>
		{
			Some(Change::replace(BrainIr::set_range(a.value, b.start, a.end)))
		}
		[BrainIr::SetRange(a), BrainIr::SetRange(b)] if a.range() == b.range() => {
			Some(Change::remove_offset(0))
		}
		[BrainIr::ChangeCell(.., offset), BrainIr::SetRange(options)]
			if options.range().contains(&offset.get_or_zero()) =>
		{
			Some(Change::remove_offset(0))
		}
		[BrainIr::SetCell(a, x), BrainIr::SetCell(b, y)] => {
			let x = x.get_or_zero();
			let y = y.get_or_zero();
			let min = cmp::min(x, y);
			let max = cmp::max(x, y);

			if !matches!((max - min).unsigned_abs(), 1) {
				return None;
			}

			let (a, b) = if x == min { (*a, *b) } else { (*b, *a) };

			Some(Change::replace(BrainIr::set_many_cells([a, b], min)))
		}
		[BrainIr::SetManyCells(options), BrainIr::SetCell(a, x)]
		| [BrainIr::SetCell(a, x), BrainIr::SetManyCells(options)] => {
			let x = x.get_or_zero();
			let range = options.range();

			if x != range.end {
				return None;
			}

			Some(Change::replace(BrainIr::set_many_cells(
				options.values.iter().copied().chain(iter::once(*a)),
				range.start,
			)))
		}
		[BrainIr::SetManyCells(a), BrainIr::SetManyCells(b)]
			if a.start == b.start && a.values.len() <= b.values.len() =>
		{
			Some(Change::remove_offset(0))
		}
		[BrainIr::SetManyCells(a), BrainIr::SetManyCells(b)]
			if a.range().end == b.range().start =>
		{
			Some(Change::replace(BrainIr::set_many_cells(
				a.values.iter().copied().chain(b.values.iter().copied()),
				a.start.get_or_zero(),
			)))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::SetRange(set_range_options),
		] if set_many_options.range().end == *set_range_options.range().start() => {
			let mut new_values = set_many_options.values.clone();

			for _ in set_range_options.range() {
				new_values.push(set_range_options.value);
			}

			Some(Change::replace(BrainIr::set_many_cells(
				new_values,
				set_many_options.start.get_or_zero(),
			)))
		}
		// [
		// 	BrainIr::SetRange(set_range_options),
		// 	BrainIr::SetManyCells(set_many_options),
		// ]
		// | [
		// 	BrainIr::SetManyCells(set_many_options),
		// 	BrainIr::SetRange(set_range_options),
		// ] => {
		// 	let set_many_range = set_many_options.range();
		// 	let set_range_range = set_range_options.range();

		// 	let set_many_count = set_many_range.len();
		// 	let set_range_count = set_range_range.count();

		// 	if set_many_count == set_range_count
		// 		&& set_many_options.start.get_or_zero() == set_range_options.start
		// 	{
		// 		Some(Change::remove_offset(0))
		// 	} else {
		// 		None
		// 	}
		// }
		[
			BrainIr::SetRange(set_range_options),
			BrainIr::SetManyCells(set_many_options),
		] => {
			let set_many_count = set_many_options.range().len();
			let set_range_count = set_range_options.range().count();

			if set_many_options.start.get_or_zero() == set_range_options.start
				&& set_many_count >= set_range_count
			{
				Some(Change::remove_offset(0))
			} else {
				None
			}
		}
		_ => None,
	}
}

pub fn optimize_mem_set_move_change(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetManyCells(options),
			BrainIr::MovePointer(x),
			BrainIr::ChangeCell(a, None),
		] => {
			let mut range = options.range();

			if !range.contains(x) {
				return None;
			}

			let cell_index = range.position(|y| y == *x)?;

			let mut values = options.values.clone();

			values[cell_index] = values[cell_index].wrapping_add_signed(*a);

			Some(Change::swap([
				BrainIr::set_many_cells(values, options.start.get_or_zero()),
				BrainIr::move_pointer(*x),
			]))
		}
		_ => None,
	}
}

pub fn optimize_duplicate_cell_vectorization(ops: [&BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::DuplicateCell { values }] => {
			if is_range(values) {
				return None;
			}

			let mut out = Vec::new();

			for w in values.windows(2) {
				let a = w[0];
				let b = w[1];

				out.push(a);

				for missing_offset in (a.offset() + 1)..b.offset() {
					out.push(CellChangeOptions::new(0, missing_offset));
				}
			}

			if let Some(last) = values.last() {
				out.push(*last);
			}

			Some(Change::replace(BrainIr::DuplicateCell { values: out }))
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
		[BrainIr::SetCell(a, None), BrainIr::DuplicateCell { values }] => {
			let mut output = Vec::with_capacity(values.len() + 1);

			output.push(BrainIr::clear_cell());

			for option in values {
				let factored_value = option.value().wrapping_mul(*a as i8);

				output.push(BrainIr::change_cell_at(factored_value, option.offset()));
			}

			Some(Change::swap(output))
		}
		_ => None,
	}
}

pub fn unroll_constant_if_nz(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(x @ 1..=u8::MAX, None),
			BrainIr::IfNotZero(ops),
		] => Some(Change::swap(
			iter::once(BrainIr::set_cell(*x)).chain(ops.iter().cloned()),
		)),
		_ => None,
	}
}
