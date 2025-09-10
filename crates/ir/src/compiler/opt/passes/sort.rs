use std::cmp;

use frick_utils::{GetOrZero as _, IteratorExt as _};

use super::{BrainIr, Change};

pub fn sort_changes(ops: &[BrainIr; 2]) -> Option<Change> {
	if !ops.iter().all(|i| {
		matches!(
			i,
			BrainIr::SetCell(..) | BrainIr::ChangeCell(..) | BrainIr::MemSet { .. }
		)
	}) {
		return None;
	}

	if ops.iter().is_sorted_by_key(sorter_key) {
		return None;
	}

	Some(Change::swap(ops.iter().cloned().sorted_by_key(sorter_key)))
}

fn sorter_key(i: &BrainIr) -> (u8, i32, i32) {
	match i {
		BrainIr::ChangeCell(.., offset) | BrainIr::SetCell(.., offset) => {
			let offset = offset.get_or_zero();

			(0, offset.abs(), offset)
		}
		BrainIr::MemSet { range, .. } => {
			let start = *range.start();
			let end = *range.end();

			let min = cmp::min(start, end);

			(2, min.abs(), min)
		}
		_ => unreachable!(),
	}
}
