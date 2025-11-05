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
			BrainIr::TakeValueTo(options) => sum = sum.wrapping_add(options.offset()),
			BrainIr::ChangeCell(..)
			| BrainIr::SetCell(..)
			| BrainIr::InputIntoCell(..)
			| BrainIr::Output(..)
			| BrainIr::MoveValueTo(..)
			| BrainIr::FetchValueFrom(..)
			| BrainIr::SubCell(..)
			| BrainIr::ReplaceValueFrom(..)
			| BrainIr::ScaleValue(..)
			| BrainIr::SetRange { .. }
			| BrainIr::SetManyCells { .. }
			| BrainIr::DuplicateCell { .. }
			| BrainIr::ChangeManyCells(..) => {}
			_ => return None,
		}
	}
	Some(sum)
}
