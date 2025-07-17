#![allow(clippy::trivially_copy_pass_by_ref)]

use alloc::vec::Vec;

use super::Change;
use crate::BrainMlir;

pub fn combine_instructions(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[BrainMlir::ChangeCell(i1), BrainMlir::ChangeCell(i2)] if *i1 == -i2 => {
			Some(Change::remove())
		}
		[BrainMlir::ChangeCell(i1), BrainMlir::ChangeCell(i2)] => Some(Change::replace(
			BrainMlir::change_cell(i1.wrapping_add(*i2)),
		)),
		[BrainMlir::MovePtr(i1), BrainMlir::MovePtr(i2)] if *i1 == -i2 => Some(Change::remove()),
		[BrainMlir::MovePtr(i1), BrainMlir::MovePtr(i2)] => {
			Some(Change::replace(BrainMlir::move_ptr(i1.wrapping_add(*i2))))
		}
		[BrainMlir::SetCell(i1), BrainMlir::ChangeCell(i2)] => {
			Some(Change::replace(BrainMlir::set_cell(i1.wrapping_add_signed(*i2))))
		}
		[BrainMlir::SetCell(_), BrainMlir::SetCell(_)] => Some(Change::remove_offset(0)),
		_ => None,
	}
}

pub const fn clear_cell(ops: &[BrainMlir; 1]) -> Option<Change> {
	match ops {
		[BrainMlir::DynamicLoop(v)] => match v.as_slice() {
			[BrainMlir::ChangeCell(-1)] => Some(Change::replace(BrainMlir::set_cell(0))),
			_ => None,
		},
		_ => None,
	}
}
