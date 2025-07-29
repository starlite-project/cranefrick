#![allow(clippy::trivially_copy_pass_by_ref)]

mod loops;
mod sort;

use alloc::vec::Vec;
use core::num::NonZero;

pub use self::{loops::*, sort::*};
use super::Change;
use crate::BrainMlir;

pub fn optimize_consecutive_instructions(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[BrainMlir::ChangeCell(a, x), BrainMlir::ChangeCell(b, y)] if *x == *y => {
			Some(Change::replace(BrainMlir::change_cell_at(
				a.wrapping_add(*b),
				x.map_or(0, NonZero::get),
			)))
		}
		[BrainMlir::MovePointer(a), BrainMlir::MovePointer(b)] => {
			Some(Change::replace(BrainMlir::move_pointer(a.wrapping_add(*b))))
		}
		_ => None,
	}
}

pub fn optimize_sets(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[
			BrainMlir::SetCell(.., a) | BrainMlir::ChangeCell(.., a),
			BrainMlir::SetCell(.., b),
		] if *a == *b => Some(Change::remove_offset(0)),
		[BrainMlir::SetCell(i1, x), BrainMlir::ChangeCell(i2, y)] if *x == *y => {
			Some(Change::replace(BrainMlir::set_cell_at(
				i1.wrapping_add_signed(*i2),
				x.map_or(0, NonZero::get),
			)))
		}
		[
			l @ BrainMlir::DynamicLoop(..),
			BrainMlir::ChangeCell(i1, None),
		] => Some(Change::swap([l.clone(), BrainMlir::set_cell(*i1 as u8)])),
		_ => None,
	}
}

pub fn clear_cell(ops: &[BrainMlir]) -> Option<Change> {
	match ops {
		[BrainMlir::ChangeCell(.., offset)] => Some(Change::replace(BrainMlir::set_cell_at(
			0,
			offset.map_or(0, NonZero::get),
		))),
		_ => None,
	}
}

pub const fn remove_noop_instructions(ops: &[BrainMlir; 1]) -> Option<Change> {
	match ops {
		[BrainMlir::ChangeCell(0, ..) | BrainMlir::MovePointer(0)] => Some(Change::remove()),
		_ => None,
	}
}

pub fn fix_beginning_instructions(ops: &mut Vec<BrainMlir>) -> bool {
	match ops.first_mut() {
		Some(BrainMlir::DynamicLoop(..)) => {
			ops.remove(0);
			true
		}
		Some(instr @ &mut BrainMlir::ChangeCell(i, x)) => {
			*instr = BrainMlir::set_cell_at(i as u8, x.map_or(0, NonZero::get));
			true
		}
		_ => false,
	}
}

pub fn add_offsets(ops: &[BrainMlir; 3]) -> Option<Change> {
	match ops {
		[
			BrainMlir::MovePointer(x),
			BrainMlir::ChangeCell(i, None),
			BrainMlir::MovePointer(y),
		] => Some(Change::swap([
			BrainMlir::change_cell_at(*i, *x),
			BrainMlir::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainMlir::MovePointer(x),
			BrainMlir::SetCell(i, None),
			BrainMlir::MovePointer(y),
		] => Some(Change::swap([
			BrainMlir::set_cell_at(*i, *x),
			BrainMlir::move_pointer(x.wrapping_add(*y)),
		])),
		_ => None,
	}
}

pub const fn optimize_scale_and_move_value(ops: &[BrainMlir]) -> Option<Change> {
	match ops {
		[
			BrainMlir::ChangeCell(-1, None),
			BrainMlir::ChangeCell(i, Some(offset)),
		] if i.is_positive() => Some(Change::replace(BrainMlir::scale_and_move_value(
			i.unsigned_abs(),
			offset.get(),
		))),
		_ => None,
	}
}

pub const fn optimize_move_value(ops: &[BrainMlir; 1]) -> Option<Change> {
	match ops {
		[BrainMlir::ScaleAndMoveValue(1, offset)] => {
			Some(Change::replace(BrainMlir::move_value(*offset)))
		}
		_ => None,
	}
}

pub const fn optimize_find_zero(ops: &[BrainMlir]) -> Option<Change> {
	match ops {
		[BrainMlir::MovePointer(offset)] => Some(Change::replace(BrainMlir::find_zero(*offset))),
		_ => None,
	}
}
