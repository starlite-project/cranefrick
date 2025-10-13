use std::iter;

use frick_utils::GetOrZero as _;

use super::{BrainIr, Change};

pub const fn optimize_sub_cell_at(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(-1, None),
			BrainIr::ChangeCell(factor @ i8::MIN..0, Some(offset)),
		]
		| [
			BrainIr::ChangeCell(factor @ i8::MIN..0, Some(offset)),
			BrainIr::ChangeCell(-1, None),
		] => Some(Change::replace(BrainIr::sub_cell_at(
			factor.unsigned_abs(),
			offset.get(),
		))),
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
		[
			..,
			BrainIr::SetCell(1..=u8::MAX, None) | BrainIr::InputIntoCell,
		] => Some(Change::remove()),
		_ => None,
	}
}

pub fn remove_empty_loops(ops: &[BrainIr]) -> Option<Change> {
	ops.is_empty().then_some(Change::remove())
}

pub fn unroll_noop_loop(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[
			BrainIr::ChangeCell(-1, None),
			BrainIr::SetCell(x, Some(offset)),
		]
		| [
			BrainIr::SetCell(x, Some(offset)),
			BrainIr::ChangeCell(-1, None),
		] => Some(Change::swap([
			BrainIr::set_cell(0),
			BrainIr::set_cell_at(*x, offset.get()),
		])),
		[
			BrainIr::ChangeCell(-1, None),
			BrainIr::SetManyCells(options),
		]
		| [
			BrainIr::SetManyCells(options),
			BrainIr::ChangeCell(-1, None),
		] if !options.range().contains(&0) => Some(Change::swap([
			BrainIr::clear_cell(),
			BrainIr::set_many_cells(options.values.iter().copied(), options.start.get_or_zero()),
		])),
		[BrainIr::ChangeCell(-1, None), BrainIr::SetRange(options)]
		| [BrainIr::SetRange(options), BrainIr::ChangeCell(-1, None)]
			if !options.range().contains(&0) =>
		{
			Some(Change::swap([
				BrainIr::clear_cell(),
				BrainIr::set_range(options.value, options.start, options.end),
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
			BrainIr::ChangeCell(-1, None),
			BrainIr::ChangeCell(i, Some(offset)),
		] if i.is_positive() => Some(Change::replace(BrainIr::move_value_to(
			i.unsigned_abs(),
			offset.get(),
		))),
		_ => None,
	}
}

pub fn optimize_duplicate_cell(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(-1, None), rest @ ..]
			if rest.iter().all(|i| matches!(i, BrainIr::ChangeCell(..))) =>
		{
			let mut values = Vec::new();

			for op in rest {
				let BrainIr::ChangeCell(value, offset) = op else {
					unreachable!()
				};

				values.push((*value, offset.get_or_zero()));
			}

			values.sort_by_key(|(.., offset)| *offset);

			Some(Change::replace(BrainIr::duplicate_cell(values)))
		}
		_ => None,
	}
}

pub fn clear_cell(ops: &[BrainIr]) -> Option<Change> {
	match ops {
		[BrainIr::ChangeCell(.., offset)] => Some(Change::replace(BrainIr::set_cell_at(
			0,
			offset.get_or_zero(),
		))),
		_ => None,
	}
}
