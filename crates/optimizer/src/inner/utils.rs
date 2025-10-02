use frick_ir::BrainIr;

pub fn calculate_ptr_movement(ops: &[BrainIr]) -> Option<i32> {
	let mut sum = 0i32;

	for op in ops {
		match op {
			BrainIr::MovePointer(offset) => {
				sum = sum.wrapping_add(*offset);
			}
			BrainIr::DynamicLoop(l) | BrainIr::IfNotZero(l) => {
				let loop_sum = calculate_ptr_movement(l)?;

				sum = sum.wrapping_add(loop_sum);
			}
			BrainIr::TakeValueTo(options) => sum += options.offset(),
			BrainIr::ChangeCell(..)
			| BrainIr::SetCell(..)
			| BrainIr::InputIntoCell
			| BrainIr::Output(..)
			| BrainIr::MoveValueTo(..)
			| BrainIr::FetchValueFrom(..)
			| BrainIr::SubCell(..)
			| BrainIr::CopyValueTo(..)
			| BrainIr::ReplaceValueFrom(..)
			| BrainIr::ScaleValue(..)
			| BrainIr::SetRange { .. }
			| BrainIr::SetManyCells { .. }
			| BrainIr::DuplicateCell { .. } => {}
			_ => return None,
		}
	}
	Some(sum)
}
