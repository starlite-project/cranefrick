//! Passes that take 4 or more ops

use frick_ir::{BrainIr, OutputOptions};
use frick_utils::Convert as _;

use crate::inner::Change;

pub fn optimize_change_write_sets(ops: [&BrainIr; 4]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(change_cell_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			out @ BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			set @ BrainIr::SetCell(set_options),
		] if change_cell_options.offset() == output_options.offset()
			&& change_cell_options.offset() == set_options.offset() =>
		{
			Some(Change::swap([
				BrainIr::output_offset_cell_at(
					change_cell_options.value(),
					change_cell_options.offset(),
				),
				out.clone().convert::<BrainIr>(),
				set.clone(),
			]))
		}
		_ => None,
	}
}

pub fn optimize_initial_mem_set_move_change(ops: [&BrainIr; 4]) -> Option<Change> {
	match ops {
		[
			BrainIr::Boundary,
			set @ BrainIr::SetManyCells(set_many_options),
			BrainIr::MovePointer(move_offset),
			BrainIr::ChangeCell(change_options),
		] if !set_many_options
			.range()
			.contains(&move_offset.wrapping_add(change_options.offset())) =>
		{
			Some(Change::swap([
				BrainIr::boundary(),
				set.clone(),
				BrainIr::move_pointer(*move_offset),
				BrainIr::set_cell_at(change_options.value() as u8, change_options.offset()),
			]))
		}
		[
			BrainIr::Boundary,
			set @ BrainIr::SetRange(set_range_options),
			BrainIr::MovePointer(move_offset),
			BrainIr::ChangeCell(change_options),
		] if !set_range_options
			.range()
			.contains(&move_offset.wrapping_add(change_options.offset())) =>
		{
			Some(Change::swap([
				BrainIr::boundary(),
				set.clone(),
				BrainIr::move_pointer(*move_offset),
				BrainIr::set_cell_at(change_options.value() as u8, change_options.offset()),
			]))
		}
		_ => None,
	}
}
