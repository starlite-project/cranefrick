#![allow(clippy::trivially_copy_pass_by_ref)]

mod loops;
mod sort;

use std::{cmp, iter};

use frick_utils::GetOrZero as _;

pub use self::{loops::*, sort::*};
use super::Change;
use crate::BrainIr;

pub fn optimize_consecutive_instructions(ops: &[BrainIr; 2]) -> Option<Change> {
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

pub fn optimize_sets(ops: &[BrainIr; 2]) -> Option<Change> {
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
		[BrainIr::SetCell(.., x), BrainIr::MemSet { range, .. }]
			if range.contains(&x.get_or_zero()) =>
		{
			Some(Change::remove_offset(0))
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

pub const fn remove_noop_instructions(ops: &[BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(0, ..) | BrainIr::MovePointer(0)] => Some(Change::remove()),
		_ => None,
	}
}

pub fn fix_beginning_instructions(ops: &mut Vec<BrainIr>) -> bool {
	match ops.first_mut() {
		Some(BrainIr::DynamicLoop(..)) => {
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

pub fn add_offsets(ops: &[BrainIr; 3]) -> Option<Change> {
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
			BrainIr::MemSet { range, value },
			BrainIr::MovePointer(y),
		] if *x == -y => Some(Change::replace(BrainIr::mem_set(
			*value,
			range.start().wrapping_add(*x)..=range.end().wrapping_add(*x),
		))),
		_ => None,
	}
}

pub fn remove_offsets(ops: &[BrainIr; 2]) -> Option<Change> {
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
		_ => None,
	}
}

pub fn optimize_move_value(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(-1, None),
			BrainIr::ChangeCell(i, Some(offset)),
		] if i.is_positive() => Some(Change::replace(BrainIr::move_value_to(
			i.unsigned_abs(),
			offset.get(),
		))),
		[BrainIr::TakeValueTo(factor, x), BrainIr::MovePointer(y)] => Some(Change::swap([
			BrainIr::move_value_to(*factor, *x),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		_ => None,
	}
}

pub const fn optimize_take_value(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::MoveValueTo(factor, x), BrainIr::MovePointer(y)] if *x == *y => {
			Some(Change::replace(BrainIr::take_value_to(*factor, *x)))
		}
		_ => None,
	}
}

pub fn optimize_fetch_value(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::MovePointer(x), BrainIr::TakeValueTo(factor, y)] => Some(Change::swap([
			BrainIr::move_pointer(x.wrapping_add(*y)),
			BrainIr::fetch_value_from(*factor, -y),
		])),
		[BrainIr::MovePointer(x), BrainIr::MoveValueTo(factor, y)] if *x == -y => {
			Some(Change::swap([
				BrainIr::fetch_value_from(*factor, *x),
				BrainIr::move_pointer(*x),
			]))
		}
		_ => None,
	}
}

pub fn optimize_replace_value(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(0, None),
			BrainIr::FetchValueFrom(factor, offset),
		] => Some(Change::replace(BrainIr::replace_value_from(
			*factor, *offset,
		))),
		[l, BrainIr::FetchValueFrom(factor, offset)] if l.is_zeroing_cell() => {
			Some(Change::swap([
				l.clone(),
				BrainIr::replace_value_from(*factor, *offset),
			]))
		}
		_ => None,
	}
}

pub fn optimize_scale_value(ops: &[BrainIr; 4]) -> Option<Change> {
	match ops {
		[
			first @ BrainIr::TakeValueTo(.., first_move),
			second @ BrainIr::TakeValueTo(.., second_move),
			BrainIr::TakeValueTo(a, third_move),
			BrainIr::TakeValueTo(b, fourth_move),
		] if *first_move == *third_move
			&& *second_move == *fourth_move
			&& *first_move == -second_move =>
		{
			Some(Change::swap([
				first.clone(),
				second.clone(),
				BrainIr::scale_value(a.wrapping_mul(*b)),
			]))
		}
		_ => None,
	}
}

pub fn optimize_writes(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(value, x),
			BrainIr::OutputCell {
				value_offset,
				offset: y,
			},
		] if *x == *y => Some(Change::swap([
			BrainIr::output_char(value.wrapping_add_signed(value_offset.get_or_zero())),
			BrainIr::set_cell_at(*value, x.get_or_zero()),
		])),
		[BrainIr::OutputChar(x), BrainIr::OutputChar(y)] => {
			Some(Change::replace(BrainIr::output_chars([*x, *y])))
		}
		[BrainIr::OutputChars(chars), BrainIr::OutputChar(c)] => Some(Change::replace(
			BrainIr::output_chars(chars.iter().copied().chain(iter::once(*c))),
		)),
		[BrainIr::OutputChars(a), BrainIr::OutputChars(b)] => Some(Change::replace(
			BrainIr::output_chars(a.iter().copied().chain(b.iter().copied())),
		)),
		_ => None,
	}
}

pub fn optimize_offset_writes(ops: &[BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(a, None),
			BrainIr::OutputCell {
				value_offset: b,
				offset: None,
			},
			BrainIr::ChangeCell(c, None),
		] if *a == -c => Some(Change::replace(BrainIr::output_offset_cell(
			a.wrapping_add(b.get_or_zero()),
		))),
		[
			BrainIr::ChangeCell(a, None),
			BrainIr::OutputCell {
				value_offset: None,
				offset: None,
			},
			BrainIr::ChangeCell(b, None),
		] => Some(Change::swap([
			BrainIr::output_offset_cell(*a),
			BrainIr::change_cell(a.wrapping_add(*b)),
		])),
		[
			BrainIr::MovePointer(x),
			BrainIr::OutputCell {
				value_offset: None,
				offset: None,
			},
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			BrainIr::output_cell_at(*x),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::ChangeCell(a, None),
			BrainIr::OutputCell {
				value_offset: b,
				offset: None,
			},
			l,
		] if l.is_zeroing_cell() => Some(Change::swap([
			BrainIr::output_offset_cell(a.wrapping_add(b.get_or_zero())),
			l.clone(),
		])),
		_ => None,
	}
}

pub const fn optimize_sets_and_writes(ops: &[BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(.., None),
			BrainIr::OutputChars(..),
			BrainIr::SetCell(.., None),
		] => Some(Change::remove_offset(0)),
		_ => None,
	}
}

pub fn optimize_constant_shifts(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(a, x), BrainIr::FetchValueFrom(factor, y)] if x.get_or_zero() == *y => {
			Some(Change::swap([
				BrainIr::set_cell_at(0, x.get_or_zero()),
				BrainIr::set_cell(a.wrapping_mul(*factor)),
			]))
		}
		[BrainIr::SetCell(a, None), BrainIr::TakeValueTo(factor, x)] => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::move_pointer(*x),
			BrainIr::change_cell(a.wrapping_mul(*factor) as i8),
		])),
		[BrainIr::SetCell(a, None), BrainIr::MoveValueTo(factor, x)] => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::change_cell_at(a.wrapping_mul(*factor) as i8, *x),
		])),
		[BrainIr::MoveValueTo(.., x), BrainIr::SetCell(a, Some(y))] if *x == y.get() => {
			Some(Change::swap([
				BrainIr::clear_cell(),
				BrainIr::set_cell_at(*a, y.get()),
			]))
		}
		_ => None,
	}
}

pub fn remove_redundant_takes(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::TakeValueTo(.., offset),
			BrainIr::SetCell(value, None),
		] => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::move_pointer(*offset),
			BrainIr::set_cell(*value),
		])),
		_ => None,
	}
}

pub const fn optimize_sub_cell(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(-1, None),
			BrainIr::ChangeCell(-1, Some(offset)),
		]
		| [
			BrainIr::ChangeCell(-1, Some(offset)),
			BrainIr::ChangeCell(-1, None),
		] => Some(Change::replace(BrainIr::sub_cell(offset.get()))),
		_ => None,
	}
}

pub fn optimize_mem_ops(ops: &[BrainIr; 2]) -> Option<Change> {
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

			Some(Change::replace(BrainIr::mem_set(*a, range)))
		}
		[BrainIr::MemSet { value: a, range }, BrainIr::SetCell(b, x)] if *a == *b => {
			let x = x.get_or_zero();
			let start = *range.start();
			let end = *range.end();

			if !matches!((x - start).unsigned_abs(), 1) && !matches!((end - x).unsigned_abs(), 1) {
				return None;
			}

			let min = cmp::min(x, start);
			let max = cmp::max(x, end);

			let range = min..=max;

			Some(Change::replace(BrainIr::mem_set(*a, range)))
		}
		[
			BrainIr::MemSet { range: x, value: a },
			BrainIr::MemSet { range: y, value: b },
		] if x.end().wrapping_add(1) == *y.start() && *a == *b => Some(Change::replace(
			BrainIr::mem_set(*a, (*x.start())..=(*y.end())),
		)),
		[
			BrainIr::MemSet { range: x, value: a },
			BrainIr::MemSet { range: y, value: b },
		] if y.end().wrapping_add(1) == *x.start() && *a == *b => Some(Change::replace(
			BrainIr::mem_set(*a, (*y.start())..=(*x.end())),
		)),
		[
			BrainIr::MemSet { range: x, .. },
			BrainIr::MemSet { range: y, .. },
		] if x == y => Some(Change::remove_offset(0)),
		[
			BrainIr::ChangeCell(.., offset) | BrainIr::SetCell(.., offset),
			BrainIr::MemSet { range, .. },
		] if range.contains(&offset.get_or_zero()) => Some(Change::remove_offset(0)),
		_ => None,
	}
}
