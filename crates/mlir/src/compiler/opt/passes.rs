#![allow(clippy::trivially_copy_pass_by_ref)]

use alloc::vec::Vec;

use super::{Change, utils::calculate_ptr_movement};
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
		[
			BrainMlir::SetCell(..) | BrainMlir::ChangeCell(.., 0),
			BrainMlir::SetCell(..),
		] => Some(Change::remove_offset(0)),
		_ => None,
	}
}

pub fn optimize_sets(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[BrainMlir::SetCell(i1), BrainMlir::ChangeCell(i2)] => Some(Change::replace(
			BrainMlir::set_cell(i1.wrapping_add_signed(*i2)),
		)),
		[l @ BrainMlir::DynamicLoop(..), BrainMlir::ChangeCell(i1)] => {
			Some(Change::swap([l.clone(), BrainMlir::set_cell(*i1 as u8)]))
		}
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

pub fn unroll_basic_loops(ops: &[BrainMlir; 2]) -> Option<Change> {
	extern crate std;

	match ops {
		[BrainMlir::ChangeCell(v), BrainMlir::DynamicLoop(l)]
			if *v >= 1 && matches!(calculate_ptr_movement(l), Some(0)) =>
		{
			if !matches!(l.as_slice(), [.., BrainMlir::ChangeCell(-1)]) {
				return None;
			}

			let mut out = Vec::with_capacity(l.len() * *v as usize);

			for _ in 0..*v {
				out.extend_from_slice(l);
			}

			out.insert(0, BrainMlir::change_cell(*v));

			out.push(BrainMlir::dynamic_loop(l.iter().cloned()));

			Some(Change::swap(out))
		}
		_ => None,
	}
}
