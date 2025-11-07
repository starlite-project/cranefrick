#![allow(non_contiguous_range_endpoints)]

use alloc::vec::Vec;
use core::iter;

use frick_ir::BrainIr;
use frick_utils::IteratorExt as _;

use crate::inner::{Change, utils::calculate_ptr_movement};

pub const fn optimize_sub_cell_at(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		&[
			BrainIr::ChangeCell(current_cell_options),
			BrainIr::ChangeCell(offset_cell_options),
		]
		| &[
			BrainIr::ChangeCell(offset_cell_options),
			BrainIr::ChangeCell(current_cell_options),
		] if matches!(
			(
				current_cell_options.into_parts(),
				offset_cell_options.into_parts()
			),
			((-1, 0), (i8::MIN..0, i32::MIN..0 | 1..=i32::MAX))
		) =>
		{
			Some(Change::replace(BrainIr::sub_cell_at(
				offset_cell_options.value().unsigned_abs(),
				offset_cell_options.offset(),
			)))
		}
		_ => None,
	}
}

pub fn remove_infinite_loops(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[.., BrainIr::SetCell(options)] if matches!(options.into_parts(), (1..=u8::MAX, 0)) => {
			Some(Change::remove())
		}
		_ => None,
	}
}

pub fn remove_empty_loops(ops: &[BrainIr]) -> Option<Change> {
	ops.is_empty().then_some(Change::remove())
}

pub const fn optimize_only_scan_tape(ops: &[BrainIr]) -> Option<Change> {
	match *ops {
		[BrainIr::MovePointer(offset)] => Some(Change::replace(BrainIr::scan_tape(0, offset, 0))),
		[BrainIr::ScanTape(scan_tape_options)] if scan_tape_options.only_scans_tape() => Some(
			Change::replace(BrainIr::scan_tape(0, scan_tape_options.scan_step(), 0)),
		),
		_ => None,
	}
}

pub fn optimize_if_nz(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[rest @ .., i] if i.is_zeroing_cell() => Some(Change::swap([BrainIr::if_not_zero(
			rest.iter().cloned().chain_once(i.clone()),
		)])),
		l @ [i, rest @ ..]
			if matches!(
				(i.is_zeroing_cell(), calculate_ptr_movement(l)),
				(true, Some(0))
			) =>
		{
			Some(Change::swap([BrainIr::if_not_zero(
				iter::once(i.clone()).chain(rest.iter().cloned()),
			)]))
		}
		_ => None,
	}
}

pub const fn optimize_move_value_from_loop(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		&[
			BrainIr::ChangeCell(current_cell_options),
			BrainIr::ChangeCell(offset_cell_options),
		]
		| &[
			BrainIr::ChangeCell(offset_cell_options),
			BrainIr::ChangeCell(current_cell_options),
		] if matches!(
			(
				current_cell_options.into_parts(),
				offset_cell_options.into_parts()
			),
			((-1, 0), (1..=i8::MAX, i32::MIN..0 | 1..=i32::MAX))
		) =>
		{
			Some(Change::replace(BrainIr::move_value_to(
				offset_cell_options.value().unsigned_abs(),
				offset_cell_options.offset(),
			)))
		}
		_ => None,
	}
}

pub fn optimize_duplicate_cell(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(options), rest @ ..]
			if rest.iter().all(|i| matches!(i, BrainIr::ChangeCell(..)))
				&& matches!(options.into_parts(), (-1, 0)) =>
		{
			let mut values = Vec::new();

			for op in rest {
				let &BrainIr::ChangeCell(options) = op else {
					unreachable!()
				};

				values.push(options.into_factor());
			}

			values.sort_by_key(|options| options.offset());

			Some(Change::replace(BrainIr::duplicate_cell(values)))
		}
		_ => None,
	}
}

pub const fn clear_cell(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		&[BrainIr::ChangeCell(options)] => {
			Some(Change::replace(BrainIr::clear_cell_at(options.offset())))
		}
		_ => None,
	}
}

pub fn unroll_nested_loops(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[l] if l.is_zeroing_cell() => Some(Change::replace(l.clone())),
		_ => None,
	}
}
