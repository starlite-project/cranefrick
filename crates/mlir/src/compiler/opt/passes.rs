#![allow(clippy::trivially_copy_pass_by_ref)]

use alloc::vec::Vec;

use super::Change;
use crate::BrainMlir;

pub fn combine_instructions(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[BrainMlir::ChangeCell(i1, x), BrainMlir::ChangeCell(i2, y)] if *i1 == -i2 && *x == *y => {
			Some(Change::remove())
		}
		[BrainMlir::ChangeCell(i1, x), BrainMlir::ChangeCell(i2, y)] if *x == *y => Some(
			Change::replace(BrainMlir::change_cell_at(i1.wrapping_add(*i2), *x)),
		),
		[BrainMlir::MovePtr(i1), BrainMlir::MovePtr(i2)] if *i1 == -i2 => Some(Change::remove()),
		[BrainMlir::MovePtr(i1), BrainMlir::MovePtr(i2)] => {
			Some(Change::replace(BrainMlir::move_ptr(i1.wrapping_add(*i2))))
		}
		[
			BrainMlir::SetCell(..) | BrainMlir::ChangeCell(.., 0),
			BrainMlir::SetCell(..),
		] => Some(Change::remove_offset(0)),
		_ => None,
	}
}

pub fn set_indices(ops: &[BrainMlir; 3]) -> Option<Change> {
	match ops {
		[
			BrainMlir::MovePtr(x),
			BrainMlir::ChangeCell(a, 0),
			BrainMlir::MovePtr(y),
		] if *x == -y => Some(Change::swap([
			// BrainMlir::change_cell_at(*a, *x),
			// BrainMlir::move_ptr(x.wrapping_add(*y)),
			BrainMlir::change_cell_at(*a, *x),
		])),
		_ => None,
	}
}

pub fn optimize_sets(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[BrainMlir::SetCell(i1), BrainMlir::ChangeCell(i2, 0)] => Some(Change::replace(
			BrainMlir::set_cell(i1.wrapping_add_signed(*i2)),
		)),
		[l @ BrainMlir::DynamicLoop(..), BrainMlir::ChangeCell(i1, 0)] => {
			Some(Change::swap([l.clone(), BrainMlir::set_cell(*i1 as u8)]))
		}
		_ => None,
	}
}

pub const fn clear_cell(ops: &[BrainMlir; 1]) -> Option<Change> {
	match ops {
		[BrainMlir::DynamicLoop(v)] => match v.as_slice() {
			[BrainMlir::ChangeCell(-1, 0)] => Some(Change::replace(BrainMlir::set_cell(0))),
			_ => None,
		},
		_ => None,
	}
}

pub const fn remove_unreachable_loops(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[
			BrainMlir::SetCell(0) | BrainMlir::DynamicLoop(..),
			BrainMlir::DynamicLoop(..),
		] => Some(Change::remove_offset(1)),
		_ => None,
	}
}

pub const fn remove_infinite_loops(ops: &[BrainMlir]) -> Option<Change> {
	match ops {
		[.., BrainMlir::SetCell(v)] if !matches!(v, 0) => Some(Change::remove()),
		[.., BrainMlir::GetInput] => Some(Change::remove()),
		_ => None,
	}
}

pub fn remove_empty_loops(ops: &[BrainMlir]) -> Option<Change> {
	ops.is_empty().then_some(Change::remove())
}

pub fn remove_early_loops(ops: &mut Vec<BrainMlir>) -> bool {
	if matches!(ops.first(), Some(BrainMlir::DynamicLoop(..))) {
		ops.remove(0);
		true
	} else {
		false
	}
}
