//! Passes that take 4 or more ops

use alloc::vec::Vec;

use frick_ir::{BrainIr, OutputOptions};

use crate::inner::Change;

pub fn optimize_change_write_sets(ops: [&BrainIr; 4]) -> Option<Change> {
	match ops {
		[
			&BrainIr::ChangeCell(change_options),
			&BrainIr::Output(OutputOptions::Cell(output_options)),
			out @ BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			set @ &BrainIr::SetCell(set_options),
		] if change_options.offset() == output_options.offset()
			&& change_options.offset() == set_options.offset() =>
		{
			Some(Change::swap([
				BrainIr::output_offset_cell_at(
					change_options.value().wrapping_add(output_options.value()),
					change_options.offset(),
				),
				out.clone(),
				set.clone(),
			]))
		}
		[
			&BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			out @ BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			set @ BrainIr::SetCell(set_options),
		] if change_options.offset() == set_options.offset() => {
			let mut new_output_options = Vec::with_capacity(output_options.len());

			for &option in output_options {
				if option.offset() == change_options.offset() {
					new_output_options.push(option.wrapping_add(change_options));
				} else {
					new_output_options.push(option);
				}
			}

			Some(Change::swap([
				BrainIr::output_cells(new_output_options),
				out.clone(),
				set.clone(),
			]))
		}
		_ => None,
	}
}

pub fn optimize_initial_set_move_change(ops: [&BrainIr; 4]) -> Option<Change> {
	match ops {
		[
			&BrainIr::Boundary,
			&BrainIr::SetCell(set_options),
			&BrainIr::MovePointer(move_offset),
			&BrainIr::ChangeCell(change_options),
		] if set_options.offset() != move_offset.wrapping_add(change_options.offset()) => {
			Some(Change::swap([
				BrainIr::boundary(),
				BrainIr::set_cell_at(set_options.value(), set_options.offset()),
				BrainIr::move_pointer(move_offset),
				BrainIr::set_cell_at(change_options.value() as u8, change_options.offset()),
			]))
		}
		[
			&BrainIr::Boundary,
			&BrainIr::SetRange(set_range_options),
			&BrainIr::MovePointer(move_offset),
			&BrainIr::ChangeCell(change_options),
		] if !set_range_options
			.range()
			.contains(&move_offset.wrapping_add(change_options.offset())) =>
		{
			Some(Change::swap([
				BrainIr::boundary(),
				BrainIr::set_range(
					set_range_options.value(),
					set_range_options.start(),
					set_range_options.end(),
				),
				BrainIr::move_pointer(move_offset),
				BrainIr::set_cell_at(change_options.value() as u8, change_options.offset()),
			]))
		}
		[
			&BrainIr::Boundary,
			BrainIr::SetManyCells(set_many_options),
			&BrainIr::MovePointer(move_offset),
			&BrainIr::ChangeCell(change_options),
		] if !set_many_options
			.range()
			.contains(&move_offset.wrapping_add(change_options.offset())) =>
		{
			Some(Change::swap([
				BrainIr::boundary(),
				BrainIr::set_many_cells(
					set_many_options.values().iter().copied(),
					set_many_options.start(),
				),
				BrainIr::move_pointer(move_offset),
				BrainIr::set_cell_at(change_options.value() as u8, change_options.offset()),
			]))
		}
		_ => None,
	}
}
