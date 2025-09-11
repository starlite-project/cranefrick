use std::cmp;

use frick_utils::{GetOrZero as _, IteratorExt as _};

use super::{BrainIr, Change};

pub fn sort_changes<const N: usize>(ops: &[BrainIr; N]) -> Option<Change> {
	if !ops.iter().all(|i| {
		matches!(
			i,
			BrainIr::SetCell(..)
				| BrainIr::ChangeCell(..)
				| BrainIr::SetRange { .. }
				| BrainIr::SetManyCells { .. }
		)
	}) {
		return None;
	}

	if ops.iter().is_sorted_by_key(sorter_key) {
		return None;
	}

	Some(Change::swap(ops.iter().cloned().sorted_by_key(sorter_key)))
}

fn sorter_key(i: &BrainIr) -> (Priority, i32, i32) {
	match i {
		BrainIr::ChangeCell(.., offset) | BrainIr::SetCell(.., offset) => {
			let offset = offset.get_or_zero();

			(Priority::High, offset.abs(), offset)
		}
		BrainIr::SetRange { range, .. } => {
			let start = *range.start();
			let end = *range.end();

			let min = cmp::min(start, end);

			(Priority::Low, min.abs(), min)
		}
		BrainIr::SetManyCells { start, .. } => {
			let start = start.get_or_zero();

			(Priority::Low, start.abs(), start)
		}
		_ => unreachable!(),
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Priority {
	High,
	Low,
}
