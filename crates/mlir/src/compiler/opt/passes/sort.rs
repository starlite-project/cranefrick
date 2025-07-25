use cranefrick_utils::IteratorExt as _;

use super::{BrainMlir, Change};

pub fn sort_changes(ops: &[BrainMlir; 2]) -> Option<Change> {
	if !ops
		.iter()
		.all(|i| matches!(i, BrainMlir::SetCell(..) | BrainMlir::ChangeCell(..)))
	{
		return None;
	}

	if ops.iter().is_sorted_by_key(sorter_key) {
		return None;
	}

	Some(Change::swap(ops.iter().cloned().sorted_by_key(sorter_key)))
}

const fn sorter_key(i: &BrainMlir) -> (i32, Option<i16>) {
	(
		match i.offset() {
			Some(offset) => offset,
			None => 0,
		},
		get_value(i),
	)
}

const fn get_value(i: &BrainMlir) -> Option<i16> {
	match i {
		BrainMlir::SetCell(i, ..) => Some(*i as i16),
		BrainMlir::ChangeCell(i, ..) => Some(*i as i16),
		_ => None,
	}
}
