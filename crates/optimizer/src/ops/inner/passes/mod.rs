mod loops;
mod peephole;

use frick_operations::{BrainOperation, BrainOperationType, CellOffsetOptions};

pub use self::{loops::*, peephole::*};

pub fn fix_beginning_instructions(ops: &mut [BrainOperation]) -> bool {
	let mut changed_any = false;

	let mut i = 0;

	let mut indices_checked = Vec::new();

	loop {
		match *ops[i].op() {
			BrainOperationType::IncrementCell(CellOffsetOptions { value, offset }) => {
				if indices_checked.contains(&offset) {
					break;
				}

				*ops[i].op_mut() = BrainOperationType::set_cell_at(value, offset);
				changed_any = true;
				indices_checked.push(offset);
			}
			BrainOperationType::DecrementCell(CellOffsetOptions { value, offset }) => {
				if indices_checked.contains(&offset) {
					break;
				}

				*ops[i].op_mut() = BrainOperationType::set_cell_at(0u8.wrapping_sub(value), offset);
				changed_any = true;
				indices_checked.push(offset);
			}
			BrainOperationType::MovePointer(offset) => {
				if indices_checked.contains(&offset) {
					break;
				}

				for i in &mut indices_checked {
					*i = i.wrapping_sub(offset);
				}
			}
			BrainOperationType::SetCell(CellOffsetOptions { offset, .. }) => {
				if indices_checked.contains(&offset) {
					break;
				}

				indices_checked.push(offset);
			}
			_ => {
				break;
			}
		}

		i += 1;
	}

	changed_any
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
