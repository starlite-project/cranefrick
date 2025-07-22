#![allow(clippy::trivially_copy_pass_by_ref)]

use alloc::vec::Vec;
use core::num::NonZero;

use itertools::Itertools as _;

use super::Change;
use crate::{BrainMlir, compiler::opt::utils::calculate_ptr_movement};

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

pub fn sort_changes(ops: &[BrainMlir; 2]) -> Option<Change> {
	if let Some(change) = sort_dynamic_changes(ops) {
		Some(change)
	} else {
		sort_static_changes(ops)
	}
}

pub fn sort_static_changes(ops: &[BrainMlir; 2]) -> Option<Change> {
	fn sorter_key(i: &BrainMlir) -> (i32, Option<u8>) {
		(i.offset().unwrap_or_default(), get_set_value(i))
	}

	const fn get_set_value(i: &BrainMlir) -> Option<u8> {
		match i {
			BrainMlir::SetCell(i, ..) => Some(*i),
			_ => None,
		}
	}

	if !ops.iter().all(|i| matches!(i, BrainMlir::SetCell(..))) {
		return None;
	}

	if ops.iter().is_sorted_by_key(sorter_key) {
		return None;
	}

	Some(Change::swap(ops.iter().cloned().sorted_by_key(sorter_key)))
}

pub fn sort_dynamic_changes(ops: &[BrainMlir; 2]) -> Option<Change> {
	fn sorter_key(i: &BrainMlir) -> (i32, Option<i8>) {
		(i.offset().unwrap_or_default(), get_change_value(i))
	}

	const fn get_change_value(i: &BrainMlir) -> Option<i8> {
		match i {
			BrainMlir::ChangeCell(i, ..) => Some(*i),
			_ => None,
		}
	}

	if !ops.iter().all(|i| matches!(i, BrainMlir::ChangeCell(..))) {
		return None;
	}

	if ops.iter().is_sorted_by_key(sorter_key) {
		return None;
	}

	Some(Change::swap(ops.iter().cloned().sorted_by_key(sorter_key)))
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

pub const fn remove_unreachable_loops(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[
			BrainMlir::SetCell(0, None) | BrainMlir::DynamicLoop(..),
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

pub const fn optimize_scale_and_move_cell(ops: &[BrainMlir]) -> Option<Change> {
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
