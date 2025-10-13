use std::iter;

use frick_ir::CellChangeOptions;
use frick_utils::GetOrZero as _;

use super::{BrainIr, Change};

pub const fn optimize_sub_cell_at(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(current_cell_options),
			BrainIr::ChangeCell(offset_cell_options),
		]
		| [
			BrainIr::ChangeCell(offset_cell_options),
			BrainIr::ChangeCell(current_cell_options),
		] if matches!(current_cell_options.into_parts(), (-1, 0))
			&& matches!(offset_cell_options.value(), i8::MIN..0)
			&& !matches!(offset_cell_options.offset(), 0) =>
		{
			Some(Change::replace(BrainIr::sub_cell_at(
				offset_cell_options.value().unsigned_abs(),
				offset_cell_options.offset(),
			)))
		}
		_ => None,
	}
}

pub fn remove_unreachable_loops(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[a, b] if a.is_zeroing_cell() && b.needs_nonzero_cell() => Some(Change::remove_offset(1)),
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
		] if matches!(change_options.into_parts(), (-1, 0))
			&& !matches!(set_options.offset(), 0) =>
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
				BrainIr::set_many_cells(
					set_options.values.iter().copied(),
					set_options.start.get_or_zero(),
				),
			]))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::SetRange(set_options),
		]
		| [
			BrainIr::SetRange(set_options),
			BrainIr::ChangeCell(change_options),
		] if matches!(change_options.into_parts(), (-1, 0))
			&& !set_options.range().contains(&0) =>
		{
			Some(Change::swap([
				BrainIr::clear_cell(),
				BrainIr::set_range(set_options.value, set_options.start, set_options.end),
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
		_ => None,
	}
}

pub const fn optimize_move_value_from_loop(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(current_cell_options),
			BrainIr::ChangeCell(offset_cell_options),
		] if matches!(current_cell_options.into_parts(), (-1, 0))
			&& matches!(offset_cell_options.value(), 1..=i8::MAX)
			&& !matches!(offset_cell_options.offset(), 0) =>
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

				values.push(*options);
			}

			values.sort_by_key(CellChangeOptions::offset);

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
