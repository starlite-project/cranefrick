use frick_operations::{BrainOperation, BrainOperationType};

pub fn is_basic_loop(ops: &[BrainOperation]) -> bool {
	ops.iter().all(|o| {
		matches!(
			o.op(),
			BrainOperationType::Comment(..)
				| BrainOperationType::OutputValue(..)
				| BrainOperationType::OutputCell(..)
				| BrainOperationType::IncrementCell(..)
				| BrainOperationType::DecrementCell(..)
				| BrainOperationType::SetCell(..)
		)
	})
}
