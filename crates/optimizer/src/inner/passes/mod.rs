use alloc::vec::Vec;

use frick_operations::{BrainOperation, BrainOperationType};

use super::Change;

pub const fn optimize_consecutive_instructions(ops: [&BrainOperation; 2]) -> Option<Change> {
	match (ops[0].op(), ops[1].op()) {
		(&BrainOperationType::ChangeCell(a), &BrainOperationType::ChangeCell(b)) => {
			Some(if a == -b {
				Change::remove()
			} else {
				Change::replace(BrainOperationType::ChangeCell(a.wrapping_add(b)))
			})
		}
		(&BrainOperationType::MovePointer(a), &BrainOperationType::MovePointer(b)) => {
			Some(if a == -b {
				Change::remove()
			} else {
				Change::replace(BrainOperationType::MovePointer(a.wrapping_add(b)))
			})
		}
		_ => None,
	}
}

pub fn optimize_set_cell_instruction(ops: [&BrainOperation; 2]) -> Option<Change> {
	match (ops[0].op(), ops[1].op()) {
		(i, &BrainOperationType::ChangeCell(value)) if i.is_zeroing_cell() => Some(Change::swap([
			ops[0].clone(),
			BrainOperation::new(BrainOperationType::SetCell(value as u8), ops[1].span()),
		])),
		_ => None,
	}
}

pub fn optimize_clear_cell_instruction(ops: &[BrainOperation]) -> Option<Change> {
	match ops {
		[op] => match op.op() {
			BrainOperationType::ChangeCell(-1) => {
				Some(Change::replace(BrainOperationType::SetCell(0)))
			}
			_ => None,
		},
		_ => None,
	}
}

pub const fn remove_comments(ops: [&BrainOperation; 1]) -> Option<Change> {
	match ops[0].op() {
		BrainOperationType::Comment(..) => Some(Change::remove()),
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
			BrainOperationType::ChangeCell(value) => {
				ops[0] = BrainOperation::new(BrainOperationType::SetCell(*value as u8), op.span());
				true
			}
			_ => false,
		},
		Some(..) | None => false,
	}
}
