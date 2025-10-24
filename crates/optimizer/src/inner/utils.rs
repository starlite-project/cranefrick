use frick_ir::BrainIr;

pub fn calculate_ptr_movement(ops: &[BrainIr]) -> Option<i32> {
	let mut sum = 0i32;

	for op in ops {
		match op {
			BrainIr::MovePointer(offset) => {
				sum = sum.wrapping_add(*offset);
			}
			BrainIr::DynamicLoop(l) | BrainIr::IfNotZero(l) => {
				if l.iter()
					.any(|op| matches!(op, BrainIr::DynamicLoop(..) | BrainIr::IfNotZero(..)))
				{
					return None;
				}

				let loop_sum = calculate_ptr_movement(l)?;

				sum = sum.wrapping_add(loop_sum);
			}
			BrainIr::ScaleAndTakeValueTo(options) => sum += options.offset(),
			BrainIr::ChangeCell(..)
			| BrainIr::SetCell(..)
			| BrainIr::InputIntoCell
			| BrainIr::Output(..)
			| BrainIr::ScaleAndMoveValueTo(..)
			| BrainIr::ScaleAndFetchValueFrom(..)
			| BrainIr::SubCell(..)
			| BrainIr::ScaleAndCopyValueTo(..)
			| BrainIr::ScaleAndReplaceValueFrom(..)
			| BrainIr::ScaleValue(..)
			| BrainIr::SetRange { .. }
			| BrainIr::SetManyCells { .. }
			| BrainIr::DuplicateCell { .. } => {}
			_ => return None,
		}
	}
	Some(sum)
}
