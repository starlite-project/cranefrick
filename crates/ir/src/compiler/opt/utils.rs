use crate::BrainIr;

pub fn calculate_ptr_movement(ops: &[BrainIr]) -> Option<i32> {
	let mut sum = 0i32;

	for op in ops {
		match op {
			BrainIr::MovePointer(offset) | BrainIr::TakeValue(.., offset) => {
				sum = sum.wrapping_add(*offset);
			}
			BrainIr::DynamicLoop(l) => {
				let loop_sum = calculate_ptr_movement(l)?;

				sum = sum.wrapping_add(loop_sum);
			}
			BrainIr::IfNz(l) => {
				let loop_sum = calculate_ptr_movement(l)?;

				sum = sum.wrapping_add(loop_sum);
			}
			BrainIr::ChangeCell(..)
			| BrainIr::SetCell(..)
			| BrainIr::InputIntoCell
			| BrainIr::OutputChar(..)
			| BrainIr::OutputCurrentCell
			| BrainIr::MoveValue(..)
			| BrainIr::FetchValue(..) => {}
			_ => return None,
		}
	}
	Some(sum)
}
