use frick_operations::{BrainOperation, BrainOperationType, CellOffsetOptions};

use crate::ops::inner::Change;

pub fn optimize_clear_cell(ops: &[BrainOperation]) -> Option<Change> {
	match ops {
		[op] => match op.op() {
			&BrainOperationType::DecrementCell(CellOffsetOptions { value, offset: 0 })
			| &BrainOperationType::IncrementCell(CellOffsetOptions { value, offset: 0 })
				if !matches!(value % 2, 0) =>
			{
				Some(Change::replace(BrainOperationType::clear_cell()))
			}
			_ => None,
		},
		_ => None,
	}
}

pub fn optimize_move_cell_value(ops: &[BrainOperation]) -> Option<Change> {
	let mapped = ops.iter().map(BrainOperation::op).collect::<Vec<_>>();

	match &*mapped {
		[
			BrainOperationType::IncrementCell(CellOffsetOptions {
				value: a,
				offset: x,
			}),
			BrainOperationType::DecrementCell(CellOffsetOptions {
				value: 1,
				offset: 0,
			}),
		]
		| [
			BrainOperationType::DecrementCell(CellOffsetOptions {
				value: 1,
				offset: 0,
			}),
			BrainOperationType::IncrementCell(CellOffsetOptions {
				value: a,
				offset: x,
			}),
		] => Some(Change::replace(BrainOperationType::MoveCellValue(
			CellOffsetOptions::new(*a, *x),
		))),
		_ => None,
	}
}

pub fn remove_infinite_loops(ops: &[BrainOperation]) -> Option<Change> {
	let mapped = ops.iter().map(BrainOperation::op).collect::<Vec<_>>();

	match &*mapped {
		[
			..,
			BrainOperationType::SetCell(CellOffsetOptions {
				value: 1..=u8::MAX,
				offset: 0,
			}),
		]
		| [BrainOperationType::InputIntoCell] => Some(Change::remove()),
		_ => None,
	}
}
