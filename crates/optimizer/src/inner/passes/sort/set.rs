#![allow(unused)]

use alloc::vec::Vec;

use frick_ir::BrainIr;
use frick_utils::IteratorExt as _;

use super::OffsetSorterKey;
use crate::inner::Change;

pub fn sort_sets<const N: usize>(ops: [&BrainIr; N]) -> Option<Change> {
	if !ops.iter().all(|i| {
		matches!(
			i,
			BrainIr::SetCell(..) | BrainIr::SetManyCells(..) | BrainIr::SetRange(..)
		)
	}) {
		return None;
	}

	if ops.iter().is_sorted_by_key(|i| sorter_key(i)) {
		return None;
	}

	let sorted = ops
		.iter()
		.map(|&i| i.clone())
		.sorted_by_key(sorter_key)
		.collect::<Vec<_>>();

	if sorted.windows(2).any(|w| w[0].offset() == w[1].offset()) {
		return None;
	}

	Some(Change::swap(sorted))
}

fn sorter_key(i: &BrainIr) -> OffsetSorterKey {
	match i {
		BrainIr::SetCell(set_options) => OffsetSorterKey(set_options.offset()),
		BrainIr::SetManyCells(set_many_options) => OffsetSorterKey(set_many_options.start()),
		BrainIr::SetRange(set_range_options) => OffsetSorterKey(set_range_options.start()),
		_ => unreachable!(),
	}
}
