use frick_operations::{BrainOperation, BrainOperationType};

pub fn is_basic_inc_dec_loop(ops: &[BrainOperation]) -> bool {
	ops.iter().all(|o| {
		matches!(
			o.op(),
			|BrainOperationType::IncrementCell(..)| BrainOperationType::DecrementCell(..)
				| BrainOperationType::SetCell(..)
		)
	})
}
