//! Miscellaneous passes that usually take 4 or more ops

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
