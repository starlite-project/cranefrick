use std::cmp::{self, Ordering};

use frick_utils::{GetOrZero as _, IteratorExt as _};

use super::{BrainIr, Change};

pub fn sort_changes<const N: usize>(ops: [&BrainIr; N]) -> Option<Change> {
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

	if ops.iter().is_sorted_by_key(|i| sorter_key(i)) {
		return None;
	}

	Some(Change::swap(
		ops.iter().map(|i| (*i).clone()).sorted_by_key(sorter_key),
	))
}

fn sorter_key(i: &BrainIr) -> Priority {
	match i {
		BrainIr::ChangeCell(.., offset) | BrainIr::SetCell(.., offset) => {
			let offset = offset.get_or_zero();

			Priority::High(offset)
		}
		BrainIr::SetRange { range, .. } => {
			let start = *range.start();
			let end = *range.end();

			let min = cmp::min(start, end);

			Priority::Low(min)
		}
		BrainIr::SetManyCells { start, .. } => {
			let start = start.get_or_zero();

			Priority::Low(start)
		}
		_ => unreachable!(),
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Priority {
	Low(i32),
	High(i32),
}

impl Ord for Priority {
	fn cmp(&self, other: &Self) -> Ordering {
		match (self, other) {
			(Self::High(..), Self::Low(..)) => Ordering::Less,
			(Self::Low(..), Self::High(..)) => Ordering::Greater,
			(Self::Low(lhs), Self::Low(rhs)) | (Self::High(lhs), Self::High(rhs)) => {
				lhs.abs().cmp(&rhs.abs()).then_with(|| lhs.cmp(rhs))
			}
		}
	}
}

impl PartialOrd for Priority {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
