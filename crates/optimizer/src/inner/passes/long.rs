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

pub fn optimize_offset_writes_set(ops: [&BrainIr; 4]) -> Option<Change> {
	match ops {
		[
			BrainIr::MovePointer(x),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			BrainIr::SetRange(set_range_options),
			BrainIr::MovePointer(y),
		] if *x == -y => {
			let range = set_range_options.range();

			if output_options.iter().any(|x| !range.contains(&x.offset())) {
				return None;
			}

			let mut output_options = output_options.clone();

			for option in &mut output_options {
				*option.offset_mut() = option.offset().wrapping_add(*x);
			}

			let range = range.start().wrapping_add(*x)..=range.end().wrapping_add(*x);

			Some(Change::swap([
				BrainIr::output_cells(output_options),
				BrainIr::set_range(set_range_options.value(), *range.start(), *range.end()),
			]))
		}
		_ => None,
	}
}
