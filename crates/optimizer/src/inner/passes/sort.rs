use core::cmp::Ordering;

use frick_ir::BrainIr;
use frick_utils::IteratorExt as _;

use crate::inner::Change;

pub fn sort_changes<const N: usize>(ops: [&BrainIr; N]) -> Option<Change> {
	if !ops.iter().all(|i| {
		matches!(
			i,
				| BrainIr::ChangeCell(..)
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

fn sorter_key(i: &BrainIr) -> OffsetSorterKey {
	match i {
		BrainIr::ChangeCell(change_options) => OffsetSorterKey(change_options.offset()),
		_ => unreachable!(),
	}
}

#[derive(PartialEq, Eq)]
struct OffsetSorterKey(i32);

impl Ord for OffsetSorterKey {
	fn cmp(&self, other: &Self) -> Ordering {
		let lhs = self.0;
		let rhs = other.0;

		lhs.abs().cmp(&rhs.abs()).then_with(|| lhs.cmp(&rhs))
	}
}

impl PartialOrd for OffsetSorterKey {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
