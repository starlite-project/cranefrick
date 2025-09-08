use std::iter;

use super::{BrainIr, Change};
use crate::compiler::opt::utils::calculate_ptr_movement;

pub fn remove_unreachable_loops(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[a, b] if a.is_zeroing_cell() && b.needs_nonzero_cell() => Some(Change::remove_offset(1)),
		_ => None,
	}
}

pub const fn remove_infinite_loops(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[
			..,
			BrainIr::SetCell(1..=u8::MAX, None) | BrainIr::InputIntoCell,
		] => Some(Change::remove()),
		_ => None,
	}
}

pub fn remove_empty_loops(ops: &[BrainIr]) -> Option<Change> {
	ops.is_empty().then_some(Change::remove())
}

pub fn unroll_basic_dynamic_loop(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(v, None), BrainIr::DynamicLoop(l)]
			if *v >= 1
				&& matches!(calculate_ptr_movement(l)?, 0)
				&& matches!(l.as_slice(), [.., BrainIr::ChangeCell(i8::MIN..=-1, None)]) =>
		{
			if l.iter().any(|op| matches!(op, BrainIr::DynamicLoop(..))) {
				return None;
			}

			let (without_decrement, decrement) = {
				let mut owned = l.clone();
				let decrement = owned.pop()?;

				let BrainIr::ChangeCell(x, None) = decrement else {
					return None;
				};
				(owned, x)
			};

			let mut out = Vec::with_capacity(without_decrement.len() * *v as usize);

			for _ in (0..*v).step_by(decrement.unsigned_abs() as usize) {
				out.extend_from_slice(&without_decrement);
			}

			Some(Change::swap(out))
		}
		[BrainIr::SetCell(v, None), BrainIr::DynamicLoop(l)]
			if *v >= 1
				&& matches!(calculate_ptr_movement(l)?, 0)
				&& matches!(l.as_slice(), [BrainIr::ChangeCell(i8::MIN..=-1, None), ..]) =>
		{
			if l.iter().any(|op| matches!(op, BrainIr::DynamicLoop(..))) {
				return None;
			}

			let (without_decrement, decrement) = {
				let mut owned = l.clone();
				let decrement = owned.remove(0);

				let BrainIr::ChangeCell(x, None) = decrement else {
					return None;
				};

				(owned, x)
			};

			let mut out = Vec::with_capacity(without_decrement.len() * *v as usize);

			for _ in (0..*v).step_by(decrement.unsigned_abs() as usize) {
				out.extend_from_slice(&without_decrement);
			}

			Some(Change::swap(out))
		}
		_ => None,
	}
}

pub fn unroll_noop_loop(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(-1, None),
			BrainIr::SetCell(x, Some(offset)),
		]
		| [
			BrainIr::SetCell(x, Some(offset)),
			BrainIr::ChangeCell(-1, None),
		] => Some(Change::swap([
			BrainIr::set_cell(0),
			BrainIr::set_cell_at(*x, offset.get()),
		])),
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

pub fn optimize_if_nz(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[rest @ .., i] if i.is_zeroing_cell() => Some(Change::swap([BrainIr::if_not_zero(
			rest.iter().cloned().chain(iter::once(i.clone())),
		)])),
		_ => None,
	}
}
