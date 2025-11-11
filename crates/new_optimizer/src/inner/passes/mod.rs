use frick_operations::{BrainOperation, BrainOperationType};

use super::Change;

pub const fn optimize_consecutive_instructions(ops: [&BrainOperation; 2]) -> Option<Change> {
	match (ops[0].ty(), ops[1].ty()) {
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

pub const fn remove_comments(ops: [&BrainOperation; 1]) -> Option<Change> {
	match ops[0].ty() {
		BrainOperationType::Comment(..) => Some(Change::remove()),
		_ => None,
	}
}
