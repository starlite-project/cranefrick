use cranefrick_utils::IteratorExt as _;

use super::{BrainIr, Change};

pub fn sort_changes(ops: &[BrainIr; 2]) -> Option<Change> {
	if !ops
		.iter()
		.all(|i| matches!(i, BrainIr::SetCell(..) | BrainIr::ChangeCell(..)))
	{
		return None;
	}

	if ops.iter().is_sorted_by_key(sorter_key) {
		return None;
	}

	Some(Change::swap(ops.iter().cloned().sorted_by_key(sorter_key)))
}

const fn sorter_key(i: &BrainIr) -> (i32, i32) {
	match i.offset() {
		None => (0, 0),
		Some(offset) => (offset.abs(), offset),
	}
}
