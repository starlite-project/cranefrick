use alloc::{borrow::ToOwned as _, vec::Vec};
use core::cmp;

use frick_ir::{BrainIr, SetManyCellsOptions, SubOptions};
use frick_utils::{
	ContainsRange as _, Convert as _, GetOrZero as _, InsertOrPush as _, IteratorExt as _,
};

use crate::inner::Change;

pub fn optimize_mem_sets(ops: [&BrainIr; 2]) -> Option<Change> {
	match ops {
		[BrainIr::SetCell(a), BrainIr::SetCell(b)] if a.value() == b.value() => {
			let x = a.offset();
			let y = b.offset();

			let min = cmp::min(x, y);
			let max = cmp::max(x, y);

			if !matches!((max - min).unsigned_abs(), 1) {
				return None;
			}

			Some(Change::replace(BrainIr::set_range(a.value(), min, max)))
		}
		[
			BrainIr::SetCell(set_options),
			BrainIr::SetRange(set_range_options),
		]
		| [
			BrainIr::SetRange(set_range_options),
			BrainIr::SetCell(set_options),
		] => {
			let x = set_options.offset();
			let range = set_range_options.range();
			let start = *range.start();
			let end = *range.end();

			if !matches!((x - start).unsigned_abs(), 1) && !matches!((end - x).unsigned_abs(), 1) {
				return None;
			}

			let min = cmp::min(x, start);

			if set_range_options.value() == set_options.value() {
				let max = cmp::max(x, end);

				Some(Change::replace(BrainIr::set_range(
					set_range_options.value(),
					min,
					max,
				)))
			} else {
				let mut values = range
					.clone()
					.map(|_| set_range_options.value())
					.collect::<Vec<_>>();

				let new_offset_raw = x.wrapping_add(min.abs());

				assert!((0..=i32::MAX).contains(&new_offset_raw));

				let new_offset = new_offset_raw as usize;

				if range.contains(&x) {
					if new_offset >= values.len() {
						values.push(set_options.value());
					} else {
						values[new_offset] = set_options.value();
					}
				} else {
					values.insert_or_push(new_offset, set_options.value());
				}

				Some(Change::replace(BrainIr::set_many_cells(values, min)))
			}
		}
		[BrainIr::SetRange(a), BrainIr::SetRange(b)]
			if a.end().wrapping_add(1) == b.start() && a.value() == b.value() =>
		{
			Some(Change::replace(BrainIr::set_range(
				a.value(),
				a.start(),
				b.end(),
			)))
		}
		[BrainIr::SetRange(a), BrainIr::SetRange(b)]
			if b.end().wrapping_add(1) == a.start() && a.value() == b.value() =>
		{
			Some(Change::replace(BrainIr::set_range(
				a.value(),
				b.start(),
				a.end(),
			)))
		}
		[BrainIr::SetRange(a), BrainIr::SetRange(b)] if a.range() == b.range() => {
			Some(Change::remove_offset(0))
		}
		[
			BrainIr::ChangeCell(change_options),
			BrainIr::SetRange(set_range_options),
		] if set_range_options.range().contains(&change_options.offset()) => {
			Some(Change::remove_offset(0))
		}
		[BrainIr::SetCell(a), BrainIr::SetCell(b)] => {
			let x = a.offset();
			let y = b.offset();
			let min = cmp::min(x, y);
			let max = cmp::max(x, y);

			if !matches!((max - min).unsigned_abs(), 1) {
				return None;
			}

			let (a, b) = if x == min {
				(a.value(), b.value())
			} else {
				(b.value(), a.value())
			};

			Some(Change::replace(BrainIr::set_many_cells([a, b], min)))
		}
		[
			BrainIr::SetCell(set_options),
			BrainIr::SetManyCells(set_many_options),
		]
		| [
			BrainIr::SetManyCells(set_many_options),
			BrainIr::SetCell(set_options),
		] if set_many_options.range().contains(&set_options.offset()) => {
			let mut set_many_options = set_many_options.clone();

			if !set_many_options.set_value_at(set_options.offset(), set_options.value()) {
				return None;
			}

			Some(Change::replace(set_many_options.convert::<BrainIr>()))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::SetCell(set_options),
		]
		| [
			BrainIr::SetCell(set_options),
			BrainIr::SetManyCells(set_many_options),
		] => {
			let x = set_options.offset();
			let range = set_many_options.range();

			if x != range.end {
				return None;
			}

			Some(Change::replace(BrainIr::set_many_cells(
				set_many_options
					.values()
					.iter()
					.copied()
					.chain_once(set_options.value()),
				range.start,
			)))
		}
		[BrainIr::SetManyCells(a), BrainIr::SetManyCells(b)]
			if a.start() == b.start() && a.values().len() <= b.values().len() =>
		{
			Some(Change::remove_offset(0))
		}
		[BrainIr::SetManyCells(a), BrainIr::SetManyCells(b)]
			if a.range().end == b.range().start =>
		{
			Some(Change::replace(BrainIr::set_many_cells(
				a.values().iter().copied().chain(b.values().iter().copied()),
				a.start().get_or_zero(),
			)))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::SetRange(set_range_options),
		] if set_many_options.range().end == *set_range_options.range().start() => {
			let mut new_values = set_many_options.values().to_owned();

			for _ in set_range_options.range() {
				new_values.push(set_range_options.value());
			}

			Some(Change::replace(BrainIr::set_many_cells(
				new_values,
				set_many_options.start(),
			)))
		}
		[
			BrainIr::SetRange(set_range_options),
			BrainIr::SetManyCells(set_many_options),
		] => {
			let set_many_count = set_many_options.range().len();
			let set_range_count = set_range_options.range().count();

			if set_many_options.start() == set_range_options.start()
				&& set_many_count >= set_range_count
			{
				Some(Change::remove_offset(0))
			} else {
				None
			}
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::MovePointer(x),
		] if set_many_options.start() == *x => Some(Change::swap([
			BrainIr::move_pointer(*x),
			BrainIr::set_many_cells(set_many_options.values().iter().copied(), 0),
		])),
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::FetchValueFrom(fetch_options),
		] if set_many_options.range().contains(&fetch_options.offset()) => {
			let fetched_value = set_many_options.value_at(fetch_options.offset())?;

			let current_cell = set_many_options.value_at(0)?;

			let mut set_many_options = set_many_options.clone();

			if !set_many_options.set_value_at(fetch_options.offset(), 0) {
				return None;
			}

			let scaled_fetched_value = fetched_value.wrapping_mul(fetch_options.factor());

			let added_value = current_cell.wrapping_add(scaled_fetched_value);

			if !set_many_options.set_value_at(0, added_value) {
				return None;
			}

			Some(Change::replace(set_many_options.convert::<BrainIr>()))
		}
		[
			BrainIr::SubCell(SubOptions::FromCell(sub_options)),
			BrainIr::SetManyCells(set_many_options),
		] => {
			let range = set_many_options.range();

			if !range.contains(&0) || !range.contains(&sub_options.offset()) {
				return None;
			}

			Some(Change::remove_offset(0))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::ChangeCell(change_options),
		] if set_many_options.range().contains(&change_options.offset()) => {
			let mut set_many_options = set_many_options.clone();

			let value_at_change_offset = set_many_options.value_at(change_options.offset())?;

			let new_value_to_set =
				value_at_change_offset.wrapping_add_signed(change_options.value());

			if !set_many_options.set_value_at(change_options.offset(), new_value_to_set) {
				return None;
			}

			Some(Change::replace(set_many_options.convert::<BrainIr>()))
		}
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::SetRange(set_range_options),
		] => {
			let set_many_range = set_many_options.range();
			let set_range_range = set_range_options.range();

			if !set_many_range.contains_range(&set_range_range) {
				return None;
			}

			let mut set_many_options = set_many_options.clone();

			for offset in set_range_range {
				if !set_many_options.set_value_at(offset, set_range_options.value()) {
					return None;
				}
			}

			Some(Change::replace(set_many_options.convert::<BrainIr>()))
		}
		_ => None,
	}
}

pub fn optimize_mem_set_move_change(ops: [&BrainIr; 3]) -> Option<Change> {
	match ops {
		[
			BrainIr::SetManyCells(set_many_options),
			BrainIr::MovePointer(x),
			BrainIr::ChangeCell(change_options),
		] if !change_options.is_offset() => {
			let mut range = set_many_options.range();

			if !range.contains(x) {
				return None;
			}

			let cell_index = range.position(|y| y == *x)?;

			let mut values = set_many_options.values().to_owned();

			values[cell_index] = values[cell_index].wrapping_add_signed(change_options.value());

			Some(Change::swap([
				BrainIr::set_many_cells(values, set_many_options.start()),
				BrainIr::move_pointer(*x),
			]))
		}
		[
			BrainIr::SetRange(set_range_options),
			BrainIr::MovePointer(x),
			BrainIr::ChangeCell(change_options),
		] if !change_options.is_offset() => {
			let mut set_many_options = SetManyCellsOptions::from(*set_range_options);

			if !set_many_options.set_value_at(*x, change_options.value() as u8) {
				return None;
			}

			Some(Change::swap([
				BrainIr::SetManyCells(set_many_options),
				BrainIr::move_pointer(*x),
			]))
		}
		_ => None,
	}
}

pub fn optimize_set_many_to_set_range(ops: [&BrainIr; 1]) -> Option<Change> {
	match ops {
		[BrainIr::SetManyCells(set_many_options)]
			if set_many_options.values().windows(2).all(|w| w[0] == w[1]) =>
		{
			let range = set_many_options.range();

			let new_range = range.start..=(range.end.wrapping_sub(1));

			Some(Change::replace(BrainIr::set_range(
				set_many_options.values().first().copied()?,
				*new_range.start(),
				*new_range.end(),
			)))
		}
		_ => None,
	}
}
