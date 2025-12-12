mod loops;
mod peephole;

use frick_operations::{BrainOperation, BrainOperationType, CellOffsetOptions};

pub use self::{loops::*, peephole::*};

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
		None => false,
	}
}

pub fn remove_non_io_ending_operations(ops: &mut Vec<BrainOperation>) -> bool {
	let old_len = ops.len();

	loop {
		if ops.last().is_some_and(|o| o.has_io()) {
			break;
		}

		ops.pop();
	}

	ops.len() != old_len
}
