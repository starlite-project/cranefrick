use frick_operations::{BrainOperation, BrainOperationType, CellOffsetOptions};

use super::Change;

pub fn remove_noop_ops(ops: [&BrainOperation; 1]) -> Option<Change> {
	match ops.map(BrainOperation::op) {
		[
			BrainOperationType::Comment(..)
			| &BrainOperationType::MovePointer(0)
			| &BrainOperationType::IncrementCell(CellOffsetOptions { value: 0, .. })
			| &BrainOperationType::DecrementCell(CellOffsetOptions { value: 0, .. }),
		] => Some(Change::remove()),
		_ => None,
	}
}

pub fn optimize_consecutive_ops(ops: [&BrainOperation; 2]) -> Option<Change> {
	match ops.map(BrainOperation::op) {
		[
			&BrainOperationType::IncrementCell(CellOffsetOptions {
				value: a,
				offset: x,
			}),
			&BrainOperationType::IncrementCell(CellOffsetOptions {
				value: b,
				offset: y,
			}),
		] if x == y => Some(Change::replace(BrainOperationType::increment_cell_at(
			a.wrapping_add(b),
			x,
		))),
		[
			&BrainOperationType::DecrementCell(CellOffsetOptions {
				value: a,
				offset: x,
			}),
			&BrainOperationType::DecrementCell(CellOffsetOptions {
				value: b,
				offset: y,
			}),
		] if x == y => Some(Change::replace(BrainOperationType::decrement_cell_at(
			a.wrapping_add(b),
			x,
		))),
		[
			&BrainOperationType::MovePointer(a),
			&BrainOperationType::MovePointer(b),
		] => Some(if a == -b {
			Change::remove()
		} else {
			Change::replace(BrainOperationType::MovePointer(a.wrapping_add(b)))
		}),
		_ => None,
	}
}

pub fn optimize_set_cell(ops: [&BrainOperation; 2]) -> Option<Change> {
	match ops.map(BrainOperation::op) {
		[
			i,
			&BrainOperationType::IncrementCell(CellOffsetOptions { value, offset: 0 }),
		] if i.is_zeroing_cell() => Some(Change::swap([
			ops[0].clone(),
			BrainOperation::new(BrainOperationType::set_cell(value), ops[1].span()),
		])),
		[
			&BrainOperationType::IncrementCell(CellOffsetOptions { offset: 0, .. })
			| &BrainOperationType::DecrementCell(CellOffsetOptions { offset: 0, .. })
			| &BrainOperationType::SetCell(CellOffsetOptions { offset: 0, .. }),
			&BrainOperationType::SetCell(CellOffsetOptions { offset: 0, .. }),
		] => Some(Change::remove_offset(0)),
		_ => None,
	}
}

pub fn remove_unreachable_loops(ops: [&BrainOperation; 2]) -> Option<Change> {
	match ops.map(BrainOperation::op) {
		[i, BrainOperationType::DynamicLoop(..)] if i.is_zeroing_cell() => {
			Some(Change::remove_offset(1))
		}
		_ => None,
	}
}

pub fn optimize_clear_cell(ops: &[BrainOperation]) -> Option<Change> {
	match ops {
		[op] => match op.op() {
			BrainOperationType::DecrementCell(CellOffsetOptions {
				value: 1,
				offset: 0,
			}) => Some(Change::replace(BrainOperationType::clear_cell())),
			_ => None,
		},
		_ => None,
	}
}

pub fn fix_beginning_instructions(ops: &mut Vec<BrainOperation>) -> bool {
	match ops.first() {
		Some(op) => match op.op() {
			BrainOperationType::DynamicLoop(..) => {
				ops.remove(0);
				true
			}
			&BrainOperationType::IncrementCell(CellOffsetOptions { value, offset }) => {
				ops[0] =
					BrainOperation::new(BrainOperationType::set_cell_at(value, offset), op.span());
				true
			}
			_ => false,
		},
		Some(..) | None => false,
	}
}

pub fn remove_changes_before_input(ops: [&BrainOperation; 2]) -> Option<Change> {
	match ops.map(BrainOperation::op) {
		[
			&BrainOperationType::IncrementCell(CellOffsetOptions { offset: 0, .. })
			| &BrainOperationType::DecrementCell(CellOffsetOptions { offset: 0, .. })
			| &BrainOperationType::SetCell(CellOffsetOptions { offset: 0, .. }),
			&BrainOperationType::InputIntoCell,
		] => Some(Change::remove_offset(0)),
		_ => None,
	}
}

pub fn optimize_output_value(ops: [&BrainOperation; 2]) -> Option<Change> {
	match ops.map(BrainOperation::op) {
		[
			&BrainOperationType::SetCell(CellOffsetOptions { offset: 0, value }),
			&BrainOperationType::OutputCurrentCell,
		] => Some(Change::swap([
			BrainOperation::new(BrainOperationType::OutputValue(value), ops[1].span()),
			BrainOperation::new(BrainOperationType::set_cell_at(value, 0), ops[0].span()),
		])),
		_ => None,
	}
}

pub fn add_offsets(ops: [&BrainOperation; 3]) -> Option<Change> {
	match ops.map(BrainOperation::op) {
		[
			&BrainOperationType::MovePointer(x),
			&BrainOperationType::IncrementCell(options),
			&BrainOperationType::MovePointer(y),
		] => Some(Change::swap([
			BrainOperation::new(
				BrainOperationType::increment_cell_at(
					options.value(),
					options.offset().wrapping_add(x),
				),
				ops[1].span(),
			),
			BrainOperation::new(
				BrainOperationType::MovePointer(x.wrapping_add(y)),
				ops[0].span(),
			),
		])),
		[
			&BrainOperationType::MovePointer(x),
			&BrainOperationType::DecrementCell(options),
			&BrainOperationType::MovePointer(y),
		] => Some(Change::swap([
			BrainOperation::new(
				BrainOperationType::decrement_cell_at(
					options.value(),
					options.offset().wrapping_add(x),
				),
				ops[1].span(),
			),
			BrainOperation::new(
				BrainOperationType::MovePointer(x.wrapping_add(y)),
				ops[0].span(),
			),
		])),
		[
			&BrainOperationType::MovePointer(x),
			&BrainOperationType::SetCell(options),
			&BrainOperationType::MovePointer(y),
		] => Some(Change::swap([
			BrainOperation::new(
				BrainOperationType::set_cell_at(options.value(), options.offset().wrapping_add(x)),
				ops[1].span(),
			),
			BrainOperation::new(
				BrainOperationType::MovePointer(x.wrapping_add(y)),
				ops[0].span(),
			),
		])),
		_ => None,
	}
}
