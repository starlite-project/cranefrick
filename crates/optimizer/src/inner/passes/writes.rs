use alloc::vec::Vec;
use core::iter;

use frick_ir::{BrainIr, OffsetCellOptions, OutputOptions, SetManyCellsOptions};
use frick_utils::{Convert as _, IteratorExt as _};

use crate::inner::Change;

pub fn optimize_writes(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(set_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
		] if output_options.offset() == set_options.offset() => Some(Change::swap([
			BrainIr::output_char(
				set_options
					.value()
					.wrapping_add_signed(output_options.value()),
			),
			BrainIr::set_cell_at(set_options.value(), set_options.offset()),
		])),
		[i, BrainIr::Output(OutputOptions::Cell(output_options))]
			if i.is_zeroing_cell() && !output_options.is_offset() =>
		{
			Some(Change::swap([
				i.clone(),
				BrainIr::output_char(output_options.value() as u8),
			]))
		}
		[
			BrainIr::Output(OutputOptions::Char(x)),
			BrainIr::Output(OutputOptions::Char(y)),
		] => Some(Change::replace(BrainIr::output_str([*x, *y]))),
		[
			BrainIr::Output(OutputOptions::Str(chars)),
			BrainIr::Output(OutputOptions::Char(c)),
		] => Some(Change::replace(BrainIr::output_str(
			chars.iter().chain_once(c).copied(),
		))),
		[
			BrainIr::Output(OutputOptions::Char(c)),
			BrainIr::Output(OutputOptions::Str(chars)),
		] => Some(Change::replace(BrainIr::output_str(
			iter::once(c).chain(chars.iter()).copied(),
		))),
		[
			BrainIr::Output(OutputOptions::Str(a)),
			BrainIr::Output(OutputOptions::Str(b)),
		] => Some(Change::replace(BrainIr::output_str(
			a.iter().chain(b.iter()).copied(),
		))),
		[
			BrainIr::Output(OutputOptions::Cell(a)),
			BrainIr::Output(OutputOptions::Cell(b)),
		] => Some(Change::replace(BrainIr::output_cells([*a, *b]))),
		[
			BrainIr::Output(OutputOptions::Cell(x)),
			BrainIr::Output(OutputOptions::Cells(other)),
		] => Some(Change::replace(BrainIr::output_cells(
			iter::once(x).chain(other.iter()).copied(),
		))),
		[
			BrainIr::Output(OutputOptions::Cells(other)),
			BrainIr::Output(OutputOptions::Cell(x)),
		] => Some(Change::replace(BrainIr::output_cells(
			other.iter().chain_once(x).copied(),
		))),
		[
			BrainIr::Output(OutputOptions::Cells(a)),
			BrainIr::Output(OutputOptions::Cells(b)),
		] => Some(Change::replace(BrainIr::output_cells(
			a.iter().chain(b.iter()).copied(),
		))),
		[
			BrainIr::SetCell(set_options),
			BrainIr::Output(OutputOptions::Cells(output_options)),
		] if output_options
			.iter()
			.all(|x| x.offset() == set_options.offset()) =>
		{
			let mut chars = Vec::with_capacity(output_options.len());

			for value_offset in output_options.iter().map(|x| x.value()) {
				chars.push(set_options.value().wrapping_add_signed(value_offset));
			}

			Some(Change::swap([
				BrainIr::output_str(chars),
				BrainIr::set_cell_at(set_options.value(), set_options.offset()),
			]))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::Output(OutputOptions::Cells(output_options)),
		] if output_options
			.iter()
			.all(|x| set_many_options.value_at(x.offset()).is_some()) =>
		{
			let mut str = Vec::with_capacity(output_options.len());

			for opt in output_options {
				let current_value = set_many_options.value_at(opt.offset())?;

				str.push(current_value.wrapping_add_signed(opt.value()));
			}

			Some(Change::swap([
				BrainIr::output_str(str),
				BrainIr::set_many_cells(
					set_many_options.values().iter().copied(),
					set_many_options.start(),
				),
			]))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
		] => {
			let char_at = set_many_options
				.value_at(output_options.offset())?
				.wrapping_add_signed(output_options.value());

			Some(Change::swap([
				BrainIr::output_char(char_at),
				BrainIr::set_many_cells(
					set_many_options.values().iter().copied(),
					set_many_options.start(),
				),
			]))
		}
		[
			change @ (BrainIr::SetCell(..)
			| BrainIr::ChangeCell(..)
			| BrainIr::SetManyCells(..)
			| BrainIr::SetRange(..)),
			out @ BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
		] => Some(Change::swap([out.clone(), change.clone()])),
		_ => None,
	}
}

pub fn optimize_offset_writes(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(a),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::ChangeCell(b),
		] if a.offset() == output_options.offset() && output_options.offset() == b.offset() => {
			Some(if a.value() == -b.value() {
				Change::replace(BrainIr::output_offset_cell_at(
					a.value().wrapping_add(output_options.value()),
					a.offset(),
				))
			} else {
				Change::swap([
					BrainIr::change_cell_at(a.value().wrapping_add(b.value()), a.offset()),
					BrainIr::output_offset_cell_at(
						output_options.value().wrapping_sub(b.value()),
						a.offset(),
					),
				])
			})
		}
		[
			BrainIr::MovePointer(x),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::MovePointer(y),
		] if matches!(output_options.value(), 0) => Some(Change::swap([
			BrainIr::output_cell_at(x.wrapping_add(output_options.offset())),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::MovePointer(x),
			out @ BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			out.clone(),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::SetCell(set_options),
		] if change_options.offset() == output_options.offset()
			&& output_options.offset() == set_options.offset() =>
		{
			Some(Change::swap([
				BrainIr::output_offset_cell_at(
					output_options.value().wrapping_add(change_options.value()),
					output_options.offset(),
				),
				BrainIr::set_cell_at(set_options.value(), set_options.offset()),
			]))
		}
		[
			BrainIr::ChangeCell(a),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			BrainIr::ChangeCell(b),
		] => {
			let x = a.offset();
			let y = b.offset();

			if x != -y {
				return None;
			}

			let mut output = Vec::with_capacity(output_options.len());

			for option in output_options {
				if option.offset() == x {
					output.push(option.wrapping_add(*a));
				} else {
					output.push(*option);
				}
			}

			Some(Change::swap([
				BrainIr::output_cells(output),
				BrainIr::change_cell_at(a.value().wrapping_add(b.value()), a.offset()),
			]))
		}
		[
			BrainIr::MovePointer(x),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			BrainIr::output_cells(output_options.iter().map(|option| {
				OffsetCellOptions::new(option.value(), x.wrapping_add(option.offset()))
			})),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			i,
		] if i.is_zeroing_cell() && !change_options.is_offset() => {
			let mut output = Vec::with_capacity(output_options.len());

			for option in output_options {
				if option.is_offset() {
					output.push(*option);
				} else {
					output.push(OffsetCellOptions::new(
						option.value().wrapping_add(change_options.value()),
						0,
					));
				}
			}

			Some(Change::swap([BrainIr::output_cells(output), i.clone()]))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::MovePointer(x),
			BrainIr::Output(OutputOptions::Cells(output_options)),
		] => {
			let range = set_many_options.range();

			if !range.contains(x) {
				return None;
			}

			let range = (range.start.wrapping_sub(*x))..(range.end.wrapping_sub(*x));

			if output_options.iter().any(|x| !range.contains(&x.offset())) {
				return None;
			}

			let new_set_many_options =
				SetManyCellsOptions::new(set_many_options.values().iter().copied(), range.start);

			let mut chars = Vec::with_capacity(output_options.len());

			for option in output_options {
				let char = new_set_many_options
					.value_at(option.offset())?
					.wrapping_add_signed(option.value());

				chars.push(char);
			}

			Some(Change::swap([
				BrainIr::output_str(chars),
				BrainIr::set_many_cells(
					set_many_options.values().iter().copied(),
					set_many_options.start(),
				),
				BrainIr::move_pointer(*x),
			]))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::MovePointer(x),
			BrainIr::Output(OutputOptions::Cell(output_options)),
		] => {
			let range = set_many_options.range();

			if !range.contains(x) {
				return None;
			}

			let range = (range.start.wrapping_sub(*x))..(range.end.wrapping_sub(*x));

			if !range.contains(&output_options.offset()) {
				return None;
			}

			let new_set_many_options =
				SetManyCellsOptions::new(set_many_options.values().iter().copied(), range.start);

			let char = new_set_many_options
				.value_at(output_options.offset())?
				.wrapping_add_signed(output_options.value());

			Some(Change::swap([
				BrainIr::output_char(char),
				BrainIr::set_many_cells(
					set_many_options.values().iter().copied(),
					set_many_options.start(),
				),
				BrainIr::move_pointer(*x),
			]))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			BrainIr::SetRange(set_range_options),
		] if set_range_options.range().contains(&change_options.offset()) => {
			let mut new_output_options = Vec::with_capacity(output_options.len());

			for option in output_options {
				if option.offset() == change_options.offset() {
					new_output_options.push(option.wrapping_add(*change_options));
				} else {
					new_output_options.push(*option);
				}
			}

			Some(Change::swap([
				BrainIr::output_cells(new_output_options),
				(*set_range_options).convert::<BrainIr>(),
			]))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			i,
		] if i.is_clobbering_cell()
			&& !change_options.is_offset()
			&& !output_options.is_offset() =>
		{
			Some(Change::swap([
				BrainIr::output_offset_cell(
					output_options.value().wrapping_add(change_options.value()),
				),
				i.clone(),
			]))
		}
		_ => None,
	}
}

pub fn optimize_boundary_writes(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			BrainIr::Boundary,
		] => {
			let mut new_offsets = Vec::with_capacity(output_options.len());

			for option in output_options {
				if option.offset() == change_options.offset() {
					new_offsets.push(option.wrapping_add(*change_options));
				} else {
					new_offsets.push(*option);
				}
			}

			Some(Change::swap([
				BrainIr::output_cells(new_offsets),
				BrainIr::boundary(),
			]))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::Boundary,
		] => Some(if change_options.offset() == output_options.offset() {
			Change::swap([
				BrainIr::output_offset_cell_at(
					output_options.value().wrapping_add(change_options.value()),
					output_options.offset(),
				),
				BrainIr::boundary(),
			])
		} else {
			Change::remove_offset(0)
		}),
		[
			BrainIr::MovePointer(offset),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			BrainIr::Boundary,
		] => Some(Change::swap([
			BrainIr::output_cells(output_options.iter().map(|option| {
				OffsetCellOptions::new(option.value(), offset.wrapping_add(option.offset()))
			})),
			BrainIr::boundary(),
		])),
		[
			BrainIr::MovePointer(offset),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::Boundary,
		] => Some(Change::swap([
			BrainIr::output_offset_cell_at(
				output_options.value(),
				offset.wrapping_add(output_options.offset()),
			),
			BrainIr::boundary(),
		])),
		[
			BrainIr::MovePointer(..),
			BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			BrainIr::Boundary,
		] => Some(Change::remove_offset(0)),
		[
			i,
			BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			BrainIr::Boundary,
		] if !i.has_io() && !matches!(i, BrainIr::Boundary) => Some(Change::remove_offset(0)),
		[
			BrainIr::ChangeManyCells(change_many_options),
			BrainIr::Output(OutputOptions::Cells(output_options)),
			BrainIr::Boundary,
		] => {
			let mut new_output_values = Vec::new();

			for option in output_options {
				if let Some(change_many_value) = change_many_options.value_at(option.offset()) {
					new_output_values.push(OffsetCellOptions::new(
						option.value().wrapping_add(change_many_value),
						option.offset(),
					));
				} else {
					new_output_values.push(*option);
				}
			}

			Some(Change::swap([
				BrainIr::output_cells(new_output_values),
				BrainIr::boundary(),
			]))
		}
		_ => None,
	}
}

pub fn optimize_changes_and_writes(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(a),
			BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			BrainIr::SetCell(b),
		] if a.offset() == b.offset() => Some(Change::remove_offset(0)),
		[
			first
			@ (BrainIr::SetCell(..) | BrainIr::SetManyCells { .. } | BrainIr::SetRange { .. }),
			out @ BrainIr::Output(OutputOptions::Char(..) | OutputOptions::Str(..)),
			second @ (BrainIr::SetCell(..)
			| BrainIr::SetManyCells { .. }
			| BrainIr::SetRange { .. }),
		] => Some(Change::swap([out.clone(), first.clone(), second.clone()])),
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
		[
			BrainIr::MovePointer(x),
			BrainIr::ChangeCell(change_options),
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::MovePointer(y),
		] if change_options.offset() == output_options.offset() => Some(Change::swap([
			BrainIr::move_pointer(x.wrapping_add(*y)),
			BrainIr::change_cell_at(
				change_options.value(),
				change_options.offset().wrapping_add(y.wrapping_neg()),
			),
			BrainIr::output_offset_cell_at(
				output_options.value(),
				output_options.offset().wrapping_add(y.wrapping_neg()),
			),
		])),
		_ => None,
	}
}
