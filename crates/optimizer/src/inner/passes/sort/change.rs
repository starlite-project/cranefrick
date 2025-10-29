use frick_ir::BrainIr;
use frick_utils::IteratorExt as _;

use super::OffsetSorterKey;
use crate::inner::Change;

pub fn sort_changes<const N: usize>(ops: [&BrainIr; N]) -> Option<Change> {
	if !ops
		.iter()
		.all(|i| matches!(i, BrainIr::ChangeCell(..) | BrainIr::ChangeManyCells(..)))
	{
		return None;
	}

	if ops.iter().is_sorted_by_key(|i| sorter_key(i)) {
		return None;
	}

	Some(Change::swap(
		ops.iter()
			.map(|i| (*i).clone())
			.sorted_unstable_by_key(sorter_key),
	))
}

fn sorter_key(i: &BrainIr) -> OffsetSorterKey {
	match i {
		BrainIr::ChangeCell(change_options) => OffsetSorterKey(change_options.offset()),
		BrainIr::ChangeManyCells(change_many_options) => {
			OffsetSorterKey(change_many_options.start())
		}
		_ => unreachable!(),
	}
}
