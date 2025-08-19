#![allow(clippy::trivially_copy_pass_by_ref)]

mod loops;
mod sort;

use std::{iter, num::NonZero};

pub use self::{loops::*, sort::*};
use super::Change;
use crate::BrainIr;

pub fn optimize_consecutive_instructions(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(a, x), BrainIr::ChangeCell(b, y)] if *x == *y => {
			Some(Change::replace(BrainIr::change_cell_at(
				a.wrapping_add(*b),
				x.map_or(0, NonZero::get),
			)))
		}
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
			BrainIr::set_cell_at(i1.wrapping_add_signed(*i2), x.map_or(0, NonZero::get)),
		)),
		[
			l @ (BrainIr::DynamicLoop(..) | BrainIr::IfNz(..) | BrainIr::FindZero(..)),
			BrainIr::ChangeCell(i1, None),
		] => Some(Change::swap([l.clone(), BrainIr::set_cell(*i1 as u8)])),
		_ => None,
	}
}

pub fn clear_cell(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(.., offset)] => Some(Change::replace(BrainIr::set_cell_at(
			0,
			offset.map_or(0, NonZero::get),
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
			*instr = BrainIr::set_cell_at(i as u8, x.map_or(0, NonZero::get));
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
		_ => None,
	}
}

pub fn remove_offsets(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(a, x), BrainIr::MovePointer(y)] if x.map_or(0, NonZero::get) == *y => {
			Some(Change::swap([
				BrainIr::move_pointer(*y),
				BrainIr::set_cell(*a),
			]))
		}
		_ => None,
	}
}

pub const fn optimize_move_value(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(-1, None),
			BrainIr::ChangeCell(i, Some(offset)),
		] if i.is_positive() => Some(Change::replace(BrainIr::move_value(
			i.unsigned_abs(),
			offset.get(),
		))),
		_ => None,
	}
}

pub const fn optimize_take_value(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::MoveValue(factor, x), BrainIr::MovePointer(y)] if *x == *y => {
			Some(Change::replace(BrainIr::take_value(*factor, *x)))
		}
		_ => None,
	}
}

pub fn optimize_fetch_value(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::MovePointer(x), BrainIr::TakeValue(factor, y)] if *x == -y => {
			Some(Change::replace(BrainIr::fetch_value(*factor, *x)))
		}
		[BrainIr::MovePointer(x), BrainIr::MoveValue(factor, y)] if *x == -y => {
			Some(Change::swap([
				BrainIr::fetch_value(*factor, *x),
				BrainIr::move_pointer(*x),
			]))
		}
		_ => None,
	}
}

pub const fn optimize_replace_value(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(0, None),
			BrainIr::FetchValue(factor, offset),
		] => Some(Change::replace(BrainIr::replace_value(*factor, *offset))),
		_ => None,
	}
}

pub const fn optimize_find_zero(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[BrainIr::MovePointer(offset) | BrainIr::FindZero(offset)] => {
			Some(Change::replace(BrainIr::find_zero(*offset)))
		}
		_ => None,
	}
}

pub fn optimize_writes(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(value, None), BrainIr::OutputCurrentCell] => Some(Change::swap([
			BrainIr::output_char(*value),
			BrainIr::set_cell(*value),
		])),
		[BrainIr::OutputChar(x), BrainIr::OutputChar(y)] => {
			Some(Change::replace(BrainIr::output_chars([*x, *y])))
		}
		[BrainIr::OutputChars(chars), BrainIr::OutputChar(c)] => Some(Change::replace(
			BrainIr::output_chars(chars.iter().copied().chain(iter::once(*c))),
		)),
		_ => None,
	}
}

pub fn optimize_constant_shifts(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(a, x), BrainIr::FetchValue(factor, y)]
			if x.map_or(0, NonZero::get) == *y =>
		{
			Some(Change::swap([
				BrainIr::set_cell_at(0, x.map_or(0, NonZero::get)),
				BrainIr::set_cell(a.wrapping_mul(*factor)),
			]))
		}
		[BrainIr::SetCell(a, None), BrainIr::TakeValue(factor, x)] => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::move_pointer(*x),
			BrainIr::set_cell(a.wrapping_mul(*factor)),
		])),
		_ => None,
	}
}

pub fn remove_redundant_takes(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::TakeValue(.., offset),
			BrainIr::SetCell(value, None),
		] => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::move_pointer(*offset),
			BrainIr::set_cell(*value),
		])),
		_ => None,
	}
}
