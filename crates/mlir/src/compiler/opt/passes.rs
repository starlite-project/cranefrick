#![allow(clippy::trivially_copy_pass_by_ref)]

use super::Change;
use crate::BrainMlir;

pub fn combine_instructions(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[BrainMlir::ChangeCell(i1), BrainMlir::ChangeCell(i2)] if *i1 == -i2 => {
			Some(Change::remove())
		}
		[BrainMlir::ChangeCell(i1), BrainMlir::ChangeCell(i2)] => {
			Some(Change::replace(BrainMlir::ChangeCell(i1.wrapping_add(*i2))))
		}
		[BrainMlir::MovePtr(i1), BrainMlir::MovePtr(i2)] if *i1 == -i2 => Some(Change::remove()),
		[BrainMlir::MovePtr(i1), BrainMlir::MovePtr(i2)] => {
			Some(Change::replace(BrainMlir::MovePtr(i1.wrapping_add(*i2))))
		}
		_ => None,
	}
}

pub const fn clear_cell(ops: &[BrainMlir; 3]) -> Option<Change> {
	match ops {
		[
			BrainMlir::StartLoop,
			BrainMlir::ChangeCell(-1),
			BrainMlir::EndLoop,
		] => Some(Change::replace(BrainMlir::SetCell(0))),
		_ => None,
	}
}
