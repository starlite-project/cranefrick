use std::num::NonZero;

use super::{BrainIr, Change};
use crate::compiler::opt::utils::calculate_ptr_movement;

pub const fn remove_unreachable_loops(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(0, None)
			| BrainIr::DynamicLoop(..)
			| BrainIr::MoveValue(..)
			| BrainIr::FindZero(..)
			| BrainIr::IfNz(..),
			BrainIr::DynamicLoop(..)
			| BrainIr::FindZero(..)
			| BrainIr::MoveValue(..)
			| BrainIr::IfNz(..),
		] => Some(Change::remove_offset(1)),
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

pub fn partially_unroll_basic_dynamic_loop(ops: &[BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(v, None), BrainIr::DynamicLoop(l)]
			if *v >= 1
				&& matches!(calculate_ptr_movement(l)?, 0)
				&& matches!(
					l.as_slice(),
					[.., BrainIr::ChangeCell(-1, None)] | [BrainIr::ChangeCell(-1, None), ..]
				) =>
		{
			if l.iter().any(|op| matches!(op, BrainIr::DynamicLoop(..))) {
				return None;
			}

			let mut out = Vec::with_capacity(l.len() * *v as usize);

			for _ in 0..*v {
				out.extend_from_slice(l);
			}

			out.insert(0, BrainIr::change_cell(*v));

			out.push(BrainIr::dynamic_loop(l.iter().cloned()));

			Some(Change::swap(out))
		}
		_ => None,
	}
}

pub fn optimize_if_nz(ops: &[BrainIr]) -> Option<Change> {
	// match ops {
	// 	[rest @ .., BrainIr::SetCell(0, None)] => {
	// 		Some(Change::replace(BrainIr::if_nz(rest.iter().cloned())))
	// 	}
	// 	_ => None,
	// }

	if let [rest @ .., BrainIr::SetCell(0, None)] = ops {
		return Some(Change::replace(BrainIr::if_nz(rest.iter().cloned())));
	}

	let set_count = ops
		.iter()
		.filter(|op| matches!(op, BrainIr::SetCell(0, None)))
		.count();

	if !matches!(set_count, 1) {
		return None;
	}

	if !matches!(calculate_ptr_movement(ops), Some(0)) {
		return None;
	}

	Some(Change::replace(BrainIr::if_nz(
		ops.iter()
			.filter(|op| !matches!(op, BrainIr::SetCell(0, None)))
			.cloned(),
	)))
}

pub fn unroll_noop_loop(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(-1, None), BrainIr::SetCell(x, offset)] => Some(Change::swap([
			BrainIr::set_cell(0),
			BrainIr::set_cell_at(*x, offset.map_or(0, NonZero::get)),
		])),
		_ => None,
	}
}
