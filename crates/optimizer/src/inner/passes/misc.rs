//! Miscellaneous passes that usually take 3 or more ops

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

pub fn optimize_set_move_op(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(set_options),
			BrainIr::MovePointer(move_offset),
			BrainIr::ChangeCell(change_options),
		] if move_offset.wrapping_add(change_options.offset()) == set_options.offset() => {
			Some(Change::swap([
				BrainIr::set_cell_at(
					set_options
						.value()
						.wrapping_add_signed(change_options.value()),
					set_options.offset(),
				),
				BrainIr::move_pointer(*move_offset),
			]))
		}
		[
			BrainIr::SetCell(set_options),
			BrainIr::MovePointer(move_offset),
			BrainIr::Output(OutputOptions::Cell(output_options)),
		] if move_offset.wrapping_add(output_options.offset()) == set_options.offset() => {
			Some(Change::swap([
				BrainIr::output_char(
					set_options
						.value()
						.wrapping_add_signed(output_options.value()),
				),
				BrainIr::set_cell_at(set_options.value(), set_options.offset()),
				BrainIr::move_pointer(*move_offset),
			]))
		}
		_ => None,
	}
}
