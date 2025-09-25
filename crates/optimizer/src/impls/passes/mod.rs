#![allow(clippy::trivially_copy_pass_by_ref)]

mod loops;
mod sort;

use std::{cmp, iter};

use frick_ir::{BrainIr, CellChangeOptions, OutputOptions, is_range};
use frick_utils::GetOrZero as _;

pub use self::{loops::*, sort::*};
use super::Change;

pub fn optimize_consecutive_instructions(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(a, x), BrainIr::ChangeCell(b, y)] if *x == *y => Some(
			Change::replace(BrainIr::change_cell_at(a.wrapping_add(*b), x.get_or_zero())),
		),
		[BrainIr::MovePointer(a), BrainIr::MovePointer(b)] => {
			Some(Change::replace(BrainIr::move_pointer(a.wrapping_add(*b))))
		}
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
		[BrainIr::SetCell(.., x), BrainIr::SetRange { range, .. }]
			if range.contains(&x.get_or_zero()) =>
		{
			Some(Change::remove_offset(0))
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

pub fn clear_cell(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(.., offset)] => Some(Change::replace(BrainIr::set_cell_at(
			0,
			offset.get_or_zero(),
		))),
		_ => None,
	}
}

pub const fn remove_noop_instructions(ops: [&BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(0, ..) | BrainIr::MovePointer(0)] => Some(Change::remove()),
		_ => None,
	}
}

pub fn fix_beginning_instructions(ops: &mut Vec<BrainIr>) -> bool {
	match ops.first_mut() {
		Some(l) if l.needs_nonzero_cell() => {
			ops.remove(0);
			true
		}
		Some(instr @ &mut BrainIr::ChangeCell(i, x)) => {
			*instr = BrainIr::set_cell_at(i as u8, x.get_or_zero());
			true
		}
		_ => false,
	}
}

pub fn fix_ending_instructions(ops: &mut Vec<BrainIr>) -> bool {
	let Some(last) = ops.last() else {
		return false;
	};

	if last.has_output() {
		false
	} else {
		ops.remove(ops.len() - 1);
		true
	}
}

pub fn add_offsets(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::MovePointer(x),
			BrainIr::ChangeCell(i, None),
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			BrainIr::change_cell_at(*i, *x),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::MovePointer(x),
			BrainIr::SetCell(i, None),
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			BrainIr::set_cell_at(*i, *x),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::MovePointer(x),
			BrainIr::ChangeCell(i, Some(y)),
			BrainIr::MovePointer(z),
		] => Some(Change::swap([
			BrainIr::change_cell_at(*i, x.wrapping_add(y.get())),
			BrainIr::move_pointer(x.wrapping_add(*z)),
		])),
		[
			BrainIr::MovePointer(x),
			BrainIr::SetCell(i, Some(y)),
			BrainIr::MovePointer(z),
		] => Some(Change::swap([
			BrainIr::set_cell_at(*i, x.wrapping_add(y.get())),
			BrainIr::move_pointer(x.wrapping_add(*z)),
		])),
		[
			BrainIr::MovePointer(x),
			BrainIr::SetRange { range, value },
			BrainIr::MovePointer(y),
		] if *x == -y => Some(Change::replace(BrainIr::set_range(
			*value,
			range.start().wrapping_add(*x)..=range.end().wrapping_add(*x),
		))),
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
		_ => None,
	}
}

pub fn optimize_offset_writes(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(a, None),
			BrainIr::Output(OutputOptions::Cell(options)),
			BrainIr::ChangeCell(c, None),
		] if *a == -c && matches!(options.offset(), 0) => Some(Change::replace(
			BrainIr::output_offset_cell(a.wrapping_add(options.value())),
		)),
		[
			BrainIr::ChangeCell(a, None),
			BrainIr::Output(OutputOptions::Cell(options)),
			BrainIr::ChangeCell(b, None),
		] if options.is_default() => Some(Change::swap([
			BrainIr::output_offset_cell(*a),
			BrainIr::change_cell(a.wrapping_add(*b)),
		])),
		[
			BrainIr::ChangeCell(a, None),
			BrainIr::Output(OutputOptions::Cell(options)),
			BrainIr::ChangeCell(b, None),
		] if matches!(options.offset(), 0) => Some(Change::swap([
			BrainIr::change_cell(a.wrapping_add(*b)),
			BrainIr::output_offset_cell(options.value().wrapping_sub(*b)),
		])),
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
		_ => None,
	}
}

pub const fn optimize_sets_and_writes(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(.., None),
			BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			BrainIr::SetCell(.., None),
		] => Some(Change::remove_offset(0)),
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
		[BrainIr::SubCellAt(options), BrainIr::MovePointer(y)] if options.offset() == *y => {
			Some(Change::swap([
				BrainIr::move_pointer(*y),
				BrainIr::sub_from_cell(options.value(), -y),
			]))
		}
		_ => None,
	}
}

pub fn optimize_sub_cell_from_with_set(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SubCellAt(options),
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
		[BrainIr::SetCell(a, None), BrainIr::SubCellAt(options)] => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::change_cell_at(a.wrapping_mul(options.value()) as i8, options.offset()),
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

pub fn optimize_mem_ops(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(a, x), BrainIr::SetCell(b, y)] if *a == *b => {
			let x = x.get_or_zero();
			let y = y.get_or_zero();
			let min = cmp::min(x, y);
			let max = cmp::max(x, y);

			if !matches!((max - min).unsigned_abs(), 1) {
				return None;
			}

			let range = min..=max;

			Some(Change::replace(BrainIr::set_range(*a, range)))
		}
		[
			BrainIr::SetRange { value: a, range },
			BrainIr::SetCell(b, x),
		]
		| [
			BrainIr::SetCell(b, x),
			BrainIr::SetRange { value: a, range },
		] if *a == *b => {
			let x = x.get_or_zero();
			let start = *range.start();
			let end = *range.end();

			if !matches!((x - start).unsigned_abs(), 1) && !matches!((end - x).unsigned_abs(), 1) {
				return None;
			}

			let min = cmp::min(x, start);
			let max = cmp::max(x, end);

			let range = min..=max;

			Some(Change::replace(BrainIr::set_range(*a, range)))
		}
		[
			BrainIr::SetRange { range: x, value: a },
			BrainIr::SetRange { range: y, value: b },
		] if x.end().wrapping_add(1) == *y.start() && *a == *b => Some(Change::replace(
			BrainIr::set_range(*a, (*x.start())..=(*y.end())),
		)),
		[
			BrainIr::SetRange { range: x, value: a },
			BrainIr::SetRange { range: y, value: b },
		] if y.end().wrapping_add(1) == *x.start() && *a == *b => Some(Change::replace(
			BrainIr::set_range(*a, (*y.start())..=(*x.end())),
		)),
		[
			BrainIr::SetRange { range: x, .. },
			BrainIr::SetRange { range: y, .. },
		] if x == y => Some(Change::remove_offset(0)),
		[
			BrainIr::ChangeCell(.., offset) | BrainIr::SetCell(.., offset),
			BrainIr::SetRange { range, .. },
		] if range.contains(&offset.get_or_zero()) => Some(Change::remove_offset(0)),
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
		[
			BrainIr::SetManyCells { values, start },
			BrainIr::SetCell(a, x),
		]
		| [
			BrainIr::SetCell(a, x),
			BrainIr::SetManyCells { values, start },
		] => {
			let x = x.get_or_zero();
			let start = start.get_or_zero();
			let end = start.wrapping_add_unsigned(values.len() as u32);

			if x != end {
				return None;
			}

			Some(Change::replace(BrainIr::set_many_cells(
				values.iter().copied().chain(iter::once(*a)),
				start,
			)))
		}
		[
			BrainIr::SetManyCells {
				values: a,
				start: x,
			},
			BrainIr::SetManyCells {
				values: b,
				start: y,
			},
		] if *x == *y && a.len() <= b.len() => Some(Change::remove_offset(0)),
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
