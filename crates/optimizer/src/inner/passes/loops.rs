#![allow(non_contiguous_range_endpoints)]

use alloc::vec::Vec;
use core::iter;

use frick_ir::BrainIr;

use crate::inner::{Change, utils::calculate_ptr_movement};

pub const fn optimize_sub_cell_at(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(current_cell_options),
			BrainIr::ChangeCell(offset_cell_options),
		]
		| [
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

pub const fn remove_infinite_loops(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[.., BrainIr::InputIntoCell] => Some(Change::remove()),
		[.., BrainIr::SetCell(options)] if matches!(options.into_parts(), (1..=u8::MAX, 0)) => {
			Some(Change::remove())
		}
		_ => None,
	}
}

pub fn remove_empty_loops(ops: &[BrainIr]) -> Option<Change> {
	ops.is_empty().then_some(Change::remove())
}

pub fn unroll_noop_loop(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::SetCell(set_options),
		]
		| [
			BrainIr::SetCell(set_options),
			BrainIr::ChangeCell(change_options),
		] if matches!(
			(change_options.into_parts(), set_options.is_offset()),
			((-1, 0), false)
		) =>
		{
			Some(Change::swap([
				BrainIr::set_cell(0),
				BrainIr::set_cell_at(set_options.value(), set_options.offset()),
			]))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::SetManyCells(set_options),
		]
		| [
			BrainIr::SetManyCells(set_options),
			BrainIr::ChangeCell(change_options),
		] if matches!(change_options.into_parts(), (-1, 0))
			&& !set_options.range().contains(&0) =>
		{
			Some(Change::swap([
				BrainIr::clear_cell(),
				BrainIr::set_many_cells(set_options.values().iter().copied(), set_options.start()),
			]))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::SetRange(set_range_options),
		]
		| [
			BrainIr::SetRange(set_range_options),
			BrainIr::ChangeCell(change_options),
		] if matches!(change_options.into_parts(), (-1, 0))
			&& !set_range_options.range().contains(&0) =>
		{
			Some(Change::swap([
				BrainIr::clear_cell(),
				BrainIr::set_range(
					set_range_options.value(),
					set_range_options.start(),
					set_range_options.end(),
				),
			]))
		}
		_ => None,
	}
}

pub const fn optimize_find_zero(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[BrainIr::MovePointer(offset) | BrainIr::FindZero(offset)] => {
			Some(Change::replace(BrainIr::find_zero(*offset)))
		}
		_ => None,
	}
}

pub fn optimize_if_nz(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[rest @ .., i] if i.is_zeroing_cell() => Some(Change::swap([BrainIr::if_not_zero(
			rest.iter().cloned().chain(iter::once(i.clone())),
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
		[
			BrainIr::ChangeCell(current_cell_options),
			BrainIr::ChangeCell(offset_cell_options),
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
				let BrainIr::ChangeCell(options) = op else {
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
		[BrainIr::ChangeCell(options)] => {
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
