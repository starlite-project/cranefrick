#![allow(clippy::trivially_copy_pass_by_ref)]

mod long;
mod loops;
mod mem;
mod sort;
mod writes;

use alloc::vec::Vec;
use core::iter;

use frick_ir::{BrainIr, OffsetCellOptions, OutputOptions, SubOptions};
use frick_utils::{Convert as _, IteratorExt as _};

pub use self::{long::*, loops::*, mem::*, sort::*, writes::*};
use crate::inner::{Change, utils::calculate_ptr_movement};

pub const fn optimize_consecutive_instructions(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(a), BrainIr::ChangeCell(b)] if a.offset() == b.offset() => {
			Some(if a.value() == -b.value() {
				Change::remove()
			} else {
				Change::replace(BrainIr::change_cell_at(
					a.value().wrapping_add(b.value()),
					a.offset(),
				))
			})
		}
		[BrainIr::MovePointer(a), BrainIr::MovePointer(b)] => Some(if *a == -(*b) {
			Change::remove()
		} else {
			Change::replace(BrainIr::move_pointer(a.wrapping_add(*b)))
		}),
		_ => None,
	}
}

pub fn optimize_sets(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(a), BrainIr::SetCell(b)] if a.offset() == b.offset() => {
			Some(Change::remove_offset(0))
		}
		[BrainIr::ChangeCell(a), BrainIr::SetCell(b)] if a.offset() == b.offset() => {
			Some(Change::remove_offset(0))
		}
		[BrainIr::SetCell(a), BrainIr::ChangeCell(b)] if a.offset() == b.offset() => {
			Some(Change::replace(BrainIr::set_cell_at(
				a.value().wrapping_add_signed(b.value()),
				a.offset(),
			)))
		}
		[l, BrainIr::ChangeCell(options)] if l.is_zeroing_cell() && !options.is_offset() => {
			Some(Change::swap([
				l.clone(),
				BrainIr::set_cell(options.value() as u8),
			]))
		}
		[BrainIr::SetCell(options), BrainIr::InputIntoCell] if !options.is_offset() => {
			Some(Change::remove_offset(0))
		}
		[BrainIr::ChangeCell(options), BrainIr::InputIntoCell] if !options.is_offset() => {
			Some(Change::remove_offset(0))
		}
		[l, BrainIr::SetCell(options)] if options.is_default() && l.is_zeroing_cell() => {
			Some(Change::remove_offset(1))
		}
		[
			BrainIr::SetCell(set_cell_options),
			BrainIr::SetRange(set_range_options),
		] => {
			let range = set_range_options.range();
			let x = set_cell_options.offset();

			range.contains(&x).then(|| Change::remove_offset(0))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::SetManyCells(set_many_options),
		] if set_many_options.range().contains(&change_options.offset()) => {
			Some(Change::remove_offset(0))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::SetRange(set_range_options),
		] if set_range_options.range().contains(&change_options.offset()) => {
			Some(Change::remove_offset(0))
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

pub const fn remove_noop_instructions(ops: [&BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(options)] if matches!(options.value(), 0) => Some(Change::remove()),
		[BrainIr::MovePointer(0)] => Some(Change::remove()),
		_ => None,
	}
}

pub fn fix_boundary_instructions(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::Boundary, BrainIr::ChangeCell(options)] => Some(Change::swap([
			BrainIr::boundary(),
			BrainIr::set_cell_at(options.value() as u8, options.offset()),
		])),
		[l, BrainIr::Boundary] if !l.has_output() => Some(Change::remove_offset(0)),
		[BrainIr::Boundary, BrainIr::Output(OutputOptions::Cell(..))] => Some(Change::swap([
			BrainIr::boundary(),
			BrainIr::output_char(0),
			BrainIr::set_cell(0),
		])),
		[
			BrainIr::Boundary,
			BrainIr::DynamicLoop(..)
			| BrainIr::CopyValueTo(..)
			| BrainIr::MoveValueTo(..)
			| BrainIr::FetchValueFrom(..)
			| BrainIr::ReplaceValueFrom(..),
		] => Some(Change::remove_offset(1)),
		[BrainIr::Boundary, BrainIr::TakeValueTo(take_options)] => Some(Change::swap([
			BrainIr::boundary(),
			BrainIr::move_pointer(take_options.offset()),
		])),
		[BrainIr::Boundary, BrainIr::SetCell(set_options)] if matches!(set_options.value(), 0) => {
			Some(Change::remove_offset(1))
		}
		_ => None,
	}
}

pub fn optimize_initial_sets(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::Boundary,
			BrainIr::SetCell(set_options),
			BrainIr::ChangeCell(change_options),
		] => {
			let set_offset = set_options.offset();
			let change_offset = change_options.offset();

			if set_offset + 1 != change_offset {
				return None;
			}

			Some(Change::swap([
				BrainIr::boundary(),
				BrainIr::set_many_cells(
					[set_options.value(), change_options.value() as u8],
					set_offset,
				),
			]))
		}
		[
			BrainIr::Boundary,
			BrainIr::SetManyCells(set_many_options),
			BrainIr::ChangeCell(change_options),
		] if set_many_options.range().end == change_options.offset() => {
			let mut set_many_options_values = set_many_options.values().to_vec();

			set_many_options_values.push(change_options.value() as u8);

			Some(Change::swap([
				BrainIr::boundary(),
				BrainIr::set_many_cells(set_many_options_values, set_many_options.start()),
			]))
		}
		[
			BrainIr::Boundary,
			BrainIr::MovePointer(y),
			BrainIr::ChangeCell(change_options),
		] => Some(Change::swap([
			BrainIr::boundary(),
			BrainIr::move_pointer(*y),
			BrainIr::SetCell(OffsetCellOptions::new(
				change_options.value() as u8,
				change_options.offset(),
			)),
		])),
		_ => None,
	}
}

pub fn add_offsets(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::MovePointer(x),
			BrainIr::ChangeCell(change_options),
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			BrainIr::change_cell_at(
				change_options.value(),
				x.wrapping_add(change_options.offset()),
			),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::MovePointer(x),
			BrainIr::SetCell(set_options),
			BrainIr::MovePointer(y),
		] => Some(Change::swap([
			BrainIr::set_cell_at(set_options.value(), x.wrapping_add(set_options.offset())),
			BrainIr::move_pointer(x.wrapping_add(*y)),
		])),
		[
			BrainIr::MovePointer(x),
			BrainIr::SetRange(options),
			BrainIr::MovePointer(y),
		] => {
			let start = options.start().wrapping_add(*x);
			let end = options.end().wrapping_add(*x);

			let set_range_instr = BrainIr::set_range(options.value(), start, end);

			Some(if *x == -y {
				Change::replace(set_range_instr)
			} else {
				Change::swap([set_range_instr, BrainIr::move_pointer(x.wrapping_add(*y))])
			})
		}
		_ => None,
	}
}

pub fn remove_offsets(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(set_options), BrainIr::MovePointer(x)] if set_options.offset() == *x => {
			Some(Change::swap([
				BrainIr::move_pointer(*x),
				BrainIr::set_cell(set_options.value()),
			]))
		}
		[BrainIr::ChangeCell(change_options), BrainIr::MovePointer(x)]
			if change_options.offset() == *x =>
		{
			Some(Change::swap([
				BrainIr::move_pointer(*x),
				BrainIr::change_cell(change_options.value()),
			]))
		}
		[
			BrainIr::Output(OutputOptions::Cell(output_options)),
			BrainIr::MovePointer(y),
		] if output_options.offset() == *y => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::output_offset_cell(output_options.value()),
		])),
		[
			BrainIr::Output(OutputOptions::Cells(output_options)),
			BrainIr::MovePointer(y),
		] if output_options.iter().all(|x| x.offset() == *y) => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::output_cells(
				output_options
					.iter()
					.copied()
					.map(|x| OffsetCellOptions::new(x.value(), 0)),
			),
		])),
		_ => None,
	}
}

pub fn optimize_move_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::TakeValueTo(take_options), BrainIr::MovePointer(y)] => Some(Change::swap([
			BrainIr::move_value_to(take_options.factor(), take_options.offset()),
			BrainIr::move_pointer(take_options.offset().wrapping_add(*y)),
		])),
		_ => None,
	}
}

pub fn optimize_duplicate_cell_replace_from(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::DuplicateCell { values },
			BrainIr::ReplaceValueFrom(replace_options),
		] if values
			.iter()
			.any(|x| x.offset() == replace_options.offset() && matches!(x.factor(), 1))
			&& matches!(replace_options.factor(), 1) =>
		{
			let mut values = values.clone();

			let position_of_replaced_cell = values
				.iter()
				.position(|x| x.offset() == replace_options.offset())?;

			values.remove(position_of_replaced_cell);

			let new_values = values.into_iter().chain_once(OffsetCellOptions::new(1, 0));

			Some(Change::replace(BrainIr::duplicate_cell(new_values)))
		}
		_ => None,
	}
}

pub fn optimize_move_value_from_duplicate_cells(ops: [&BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::DuplicateCell { values }] if matches!(values.len(), 1) => {
			let data = values.first().copied()?;

			let value = data.factor();
			let index = data.offset();

			if value.is_negative() {
				None
			} else {
				Some(Change::replace(BrainIr::move_value_to(
					value.unsigned_abs(),
					index,
				)))
			}
		}
		_ => None,
	}
}

pub const fn optimize_take_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::MoveValueTo(options), BrainIr::MovePointer(y)] if options.offset() == *y => Some(
			Change::replace(BrainIr::take_value_to(options.factor(), options.offset())),
		),
		_ => None,
	}
}

pub fn optimize_fetch_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::MovePointer(x), BrainIr::TakeValueTo(take_options)] => Some(Change::swap([
			BrainIr::move_pointer(x.wrapping_add(take_options.offset())),
			BrainIr::fetch_value_from(take_options.factor(), -take_options.offset()),
		])),
		[BrainIr::MovePointer(x), BrainIr::MoveValueTo(move_options)]
			if *x == -move_options.offset() =>
		{
			Some(Change::swap([
				BrainIr::fetch_value_from(move_options.factor(), *x),
				BrainIr::move_pointer(*x),
			]))
		}
		_ => None,
	}
}

pub fn optimize_replace_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[l, BrainIr::FetchValueFrom(options)] if l.is_zeroing_cell() => Some(Change::swap([
			l.clone(),
			BrainIr::ReplaceValueFrom(*options),
		])),
		[BrainIr::SetCell(set_options), BrainIr::ReplaceValueFrom(..)]
			if !set_options.is_offset() =>
		{
			Some(Change::remove_offset(0))
		}
		_ => None,
	}
}

pub fn optimize_copy_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::DuplicateCell { values },
			BrainIr::ReplaceValueFrom(options),
		] => {
			if !values
				.iter()
				.any(|v| v.offset() == options.offset() && matches!(v.factor(), 1))
			{
				return None;
			}

			if !matches!(values.len(), 2) {
				return None;
			}

			let other_move_options = {
				let offset = values
					.iter()
					.position(|x| x.offset() != options.offset() && x.factor().is_positive())?;

				values.get(offset).copied()?
			};

			Some(Change::swap([
				BrainIr::copy_value_to(
					other_move_options.factor() as u8,
					other_move_options.offset(),
				),
				BrainIr::FetchValueFrom(*options),
			]))
		}
		_ => None,
	}
}

pub fn optimize_constant_shifts(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(set_options),
			BrainIr::FetchValueFrom(fetch_options),
		] if set_options.offset() == fetch_options.offset() => Some(Change::swap([
			BrainIr::clear_cell_at(set_options.offset()),
			BrainIr::set_cell(set_options.value().wrapping_mul(fetch_options.factor())),
		])),
		[
			BrainIr::SetCell(set_options),
			BrainIr::TakeValueTo(take_options),
		] if !set_options.is_offset() => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::move_pointer(take_options.offset()),
			BrainIr::change_cell(set_options.value().wrapping_mul(take_options.factor()) as i8),
		])),
		[
			BrainIr::SetCell(set_options),
			BrainIr::MoveValueTo(move_options),
		] if !set_options.is_offset() => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::change_cell_at(
				set_options.value().wrapping_mul(move_options.factor()) as i8,
				move_options.offset(),
			),
		])),
		[
			BrainIr::MoveValueTo(move_options),
			BrainIr::SetCell(set_options),
		] if set_options.is_offset() && move_options.offset() == set_options.offset() => {
			Some(Change::swap([
				BrainIr::clear_cell(),
				BrainIr::set_cell_at(set_options.value(), set_options.offset()),
			]))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::TakeValueTo(take_options),
		] if set_many_options.range().contains(&0) => {
			let mut set_many_options = set_many_options.clone();

			if !set_many_options.set_value_at(0, 0) {
				return None;
			}

			Some(Change::swap([
				set_many_options.convert::<BrainIr>(),
				BrainIr::move_pointer(take_options.offset()),
			]))
		}
		_ => None,
	}
}

pub fn optimize_sub_cell_from(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SubCell(SubOptions::CellAt(options)),
			BrainIr::MovePointer(y),
		] if options.offset() == *y => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::sub_from_cell(options.factor(), -y),
		])),
		_ => None,
	}
}

pub fn optimize_sub_cell_from_with_set(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SubCell(SubOptions::CellAt(sub_options)),
			BrainIr::SetCell(set_options),
			BrainIr::MovePointer(y),
		] if sub_options.offset() == *y && !set_options.is_offset() => Some(Change::swap([
			BrainIr::move_pointer(*y),
			BrainIr::sub_from_cell(sub_options.factor(), -y),
			BrainIr::set_cell_at(set_options.value(), -y),
		])),
		_ => None,
	}
}

pub fn remove_redundant_shifts(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::TakeValueTo(take_options),
			BrainIr::SetCell(set_options),
		] if !set_options.is_offset() => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::move_pointer(take_options.offset()),
			BrainIr::set_cell(set_options.value()),
		])),
		[
			BrainIr::MoveValueTo(move_options),
			BrainIr::SetCell(set_options),
		] if move_options.offset() == set_options.offset() && set_options.is_offset() => {
			Some(Change::swap([
				BrainIr::clear_cell(),
				BrainIr::set_cell_at(set_options.value(), set_options.offset()),
			]))
		}
		[
			BrainIr::MoveValueTo(move_options),
			BrainIr::ReplaceValueFrom(replace_options),
		] if move_options.offset() == replace_options.offset()
			&& matches!(move_options.factor(), 1)
			&& matches!(replace_options.factor(), 1) =>
		{
			Some(Change::replace(BrainIr::fetch_value_from(
				1,
				move_options.offset(),
			)))
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

pub fn optimize_if_nz_when_zeroing(ops: [&BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::IfNotZero(ops) | BrainIr::DynamicLoop(ops)] => match &**ops {
			[x] if x.needs_nonzero_cell() && x.is_zeroing_cell() => {
				Some(Change::replace(x.clone()))
			}
			_ => None,
		},
		_ => None,
	}
}

pub fn unroll_constant_duplicate_cell(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetCell(set_options),
			BrainIr::DuplicateCell { values },
		] if !set_options.is_offset() => {
			let mut output = Vec::with_capacity(values.len() + 1);

			output.push(BrainIr::clear_cell());

			for option in values {
				let factored_value = option.factor().wrapping_mul(set_options.value() as i8);

				output.push(BrainIr::change_cell_at(factored_value, option.offset()));
			}

			Some(Change::swap(output))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::DuplicateCell { values },
		] if values
			.iter()
			.all(|x| set_many_options.value_at(x.offset()).is_some()) =>
		{
			let current_cell_value = set_many_options.value_at(0)?;

			let mut set_many_options = set_many_options.clone();

			if !set_many_options.set_value_at(0, 0) {
				return None;
			}

			for dupe_option in values.iter().copied() {
				let new_value_to_set = current_cell_value.wrapping_mul(dupe_option.factor() as u8);

				set_many_options.set_value_at(dupe_option.offset(), new_value_to_set);
			}

			Some(Change::replace(BrainIr::SetManyCells(set_many_options)))
		}
		_ => None,
	}
}

pub fn unroll_constant_if_nz(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(set_options), BrainIr::IfNotZero(ops)]
			if matches!(set_options.into_parts(), (1..=u8::MAX, 0)) =>
		{
			Some(Change::swap(
				iter::once(BrainIr::set_cell(set_options.value())).chain(ops.iter().cloned()),
			))
		}
		_ => None,
	}
}

pub fn unroll_basic_dynamic_loop(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(set_options), l @ BrainIr::DynamicLoop(ops)]
			if matches!(set_options.into_parts(), (1..=u8::MAX, 0))
				&& matches!(calculate_ptr_movement(ops)?, 0)
				&& matches!(ops.as_slice(), [.., BrainIr::ChangeCell(change_options)] if matches!(change_options.into_parts(), (i8::MIN..0, 0)))
				&& !l.loop_has_movement()? =>
		{
			if ops
				.iter()
				.any(|op| matches!(op, BrainIr::DynamicLoop(..) | BrainIr::IfNotZero(..)))
			{
				return None;
			}

			let (without_decrement, decrement) = {
				let mut owned = ops.clone();
				let decrement = owned.pop()?;

				let BrainIr::ChangeCell(change_cell_options) = decrement else {
					return None;
				};

				(owned, change_cell_options.value())
			};

			let mut out =
				Vec::with_capacity(without_decrement.len() * set_options.value() as usize);

			for _ in (0..set_options.value()).step_by(decrement.unsigned_abs() as usize) {
				out.extend_from_slice(&without_decrement);
			}

			Some(Change::swap(out))
		}
		[BrainIr::SetCell(set_options), l @ BrainIr::DynamicLoop(ops)]
			if matches!(set_options.into_parts(), (1..=u8::MAX, 0))
				&& matches!(calculate_ptr_movement(ops)?, 0)
				&& matches!(ops.as_slice(), [BrainIr::ChangeCell(change_options), ..] if matches!(change_options.into_parts(), (i8::MIN..0, 0)))
				&& !l.loop_has_movement()? =>
		{
			if ops
				.iter()
				.any(|op| matches!(op, BrainIr::DynamicLoop(..) | BrainIr::IfNotZero(..)))
			{
				return None;
			}

			let (without_decrement, decrement) = {
				let mut owned = ops.clone();
				let decrement = owned.remove(0);

				let BrainIr::ChangeCell(change_options) = decrement else {
					return None;
				};

				(owned, change_options.value())
			};

			let mut out =
				Vec::with_capacity(without_decrement.len() * set_options.value() as usize);

			for _ in (0..set_options.value()).step_by(decrement.unsigned_abs() as usize) {
				out.extend_from_slice(&without_decrement);
			}

			Some(Change::swap(out))
		}
		_ => None,
	}
}

pub fn unroll_if_nz(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			set @ BrainIr::SetManyCells(set_many_options),
			BrainIr::IfNotZero(ops),
		] if !matches!(set_many_options.value_at(0)?, 0) => Some(Change::swap(
			iter::once(set.clone()).chain(ops.iter().cloned()),
		)),
		_ => None,
	}
}

pub fn optimize_scale_value(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[
			BrainIr::TakeValueTo(first_take_options),
			BrainIr::TakeValueTo(second_take_options),
		] if matches!(
			first_take_options
				.offset()
				.wrapping_add(second_take_options.offset()),
			0
		) =>
		{
			Some(Change::swap([
				BrainIr::scale_value(first_take_options.factor()),
				BrainIr::fetch_value_from(1, first_take_options.offset()),
				BrainIr::scale_value(second_take_options.factor()),
			]))
		}
		[
			BrainIr::TakeValueTo(take_options),
			BrainIr::MoveValueTo(move_options),
		] if matches!(take_options.offset().wrapping_add(move_options.offset()), 0) => {
			Some(Change::swap([
				BrainIr::scale_value(take_options.factor()),
				BrainIr::fetch_value_from(1, take_options.offset()),
				BrainIr::scale_value(move_options.factor()),
				BrainIr::move_pointer(take_options.offset()),
			]))
		}
		[BrainIr::ScaleValue(a), BrainIr::ScaleValue(b)] => {
			Some(Change::replace(BrainIr::scale_value(a.wrapping_mul(*b))))
		}
		[
			BrainIr::ScaleValue(factor),
			BrainIr::TakeValueTo(take_options),
		] if matches!(take_options.factor(), 2..=u8::MAX) => Some(Change::swap([
			BrainIr::scale_value(factor.wrapping_mul(take_options.factor())),
			BrainIr::take_value_to(1, take_options.offset()),
		])),
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::ScaleValue(factor),
		] => {
			let value = set_many_options.value_at(0)?;

			let mut set_many_options = set_many_options.clone();

			let new_value = value.wrapping_mul(*factor);

			if !set_many_options.set_value_at(0, new_value) {
				return None;
			}

			Some(Change::replace(BrainIr::SetManyCells(set_many_options)))
		}
		[BrainIr::SetCell(set_options), BrainIr::ScaleValue(factor)]
			if !set_options.is_offset() =>
		{
			Some(Change::replace(BrainIr::set_cell(
				set_options.value().wrapping_mul(*factor),
			)))
		}
		_ => None,
	}
}

pub fn optimize_initial_change_to_sets<const N: usize>(ops: [&BrainIr; N]) -> Option<Change> {
	match ops.as_slice() {
		[
			BrainIr::Boundary,
			rest @ ..,
			BrainIr::ChangeCell(change_options),
		] if rest
			.iter()
			.all(|x| matches!(x, BrainIr::ChangeCell(..) | BrainIr::SetCell(..))) =>
		{
			Some(Change::swap(
				iter::once(BrainIr::boundary())
					.chain(rest.iter().map(|x| (*x).clone()))
					.chain_once(BrainIr::set_cell_at(
						change_options.value() as u8,
						change_options.offset(),
					)),
			))
		}
		_ => None,
	}
}
