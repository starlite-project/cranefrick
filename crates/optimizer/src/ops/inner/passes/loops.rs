use frick_operations::{BrainOperation, BrainOperationType, CellOffsetOptions};

use crate::ops::inner::Change;

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
