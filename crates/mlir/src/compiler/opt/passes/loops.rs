use alloc::vec::Vec;

use super::{BrainMlir, Change};
use crate::compiler::opt::utils::calculate_ptr_movement;

pub const fn remove_unreachable_loops(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[
			BrainMlir::SetCell(0, None)
			| BrainMlir::DynamicLoop(..)
			| BrainMlir::ScaleAndMoveValue(..)
			| BrainMlir::MoveValue(..),
			BrainMlir::DynamicLoop(..),
		] => Some(Change::remove_offset(1)),
		_ => None,
	}
}

pub const fn remove_infinite_loops(ops: &[BrainMlir]) -> Option<Change> {
	match ops {
		[
			..,
			BrainMlir::SetCell(1..=u8::MAX, None) | BrainMlir::GetInput,
		] => Some(Change::remove()),
		_ => None,
	}
}

pub fn remove_empty_loops(ops: &[BrainMlir]) -> Option<Change> {
	ops.is_empty().then_some(Change::remove())
}

pub fn unroll_basic_dynamic_loop(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[BrainMlir::SetCell(v, None), BrainMlir::DynamicLoop(l)]
			if *v >= 1
				&& matches!(calculate_ptr_movement(l)?, 0)
				&& matches!(l.as_slice(), [.., BrainMlir::ChangeCell(-1, None)]) =>
		{
			if l.iter().any(|op| matches!(op, BrainMlir::DynamicLoop(..))) {
				return None;
			}

			let without_decrement = {
				let mut owned = l.clone();
				owned.pop();
				owned
			};

			let mut out = Vec::with_capacity(without_decrement.len() * *v as usize);

			for _ in 0..*v {
				out.extend_from_slice(&without_decrement);
			}

			Some(Change::swap(out))
		}
		[BrainMlir::SetCell(v, None), BrainMlir::DynamicLoop(l)]
			if *v >= 1
				&& matches!(calculate_ptr_movement(l)?, 0)
				&& matches!(l.as_slice(), [BrainMlir::ChangeCell(-1, None), ..]) =>
		{
			if l.iter().any(|op| matches!(op, BrainMlir::DynamicLoop(..))) {
				return None;
			}

			let without_decrement = {
				let mut owned = l.clone();
				owned.remove(0);
				owned
			};

			let mut out = Vec::with_capacity(without_decrement.len() * *v as usize);

			for _ in 0..*v {
				out.extend_from_slice(&without_decrement);
			}

			Some(Change::swap(out))
		}
		_ => None,
	}
}

pub fn partially_unroll_basic_dynamic_loop(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[BrainMlir::ChangeCell(v, None), BrainMlir::DynamicLoop(l)]
			if *v >= 1
				&& matches!(calculate_ptr_movement(l)?, 0)
				&& matches!(
					l.as_slice(),
					[.., BrainMlir::ChangeCell(-1, None)] | [BrainMlir::ChangeCell(-1, None), ..]
				) =>
		{
			if l.iter().any(|op| matches!(op, BrainMlir::DynamicLoop(..))) {
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

pub fn optimize_if_nz(ops: &[BrainMlir]) -> Option<Change> {
	match ops {
		[rest @ .., BrainMlir::SetCell(0, None)] => {
			Some(Change::replace(BrainMlir::if_nz(rest.iter().cloned())))
		}
		_ => None,
	}
}
