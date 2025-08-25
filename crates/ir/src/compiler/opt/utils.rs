use crate::BrainIr;

pub fn calculate_ptr_movement(ops: &[BrainIr]) -> Option<i32> {
	let mut sum = 0i32;

	for op in ops {
		match op {
			BrainIr::MovePointer(offset) | BrainIr::TakeValueTo(.., offset) => {
				sum = sum.wrapping_add(*offset);
			}
			BrainIr::DynamicLoop(l) | BrainIr::IfNotZero(l) => {
				let loop_sum = calculate_ptr_movement(l)?;

				sum = sum.wrapping_add(loop_sum);
			}
			BrainIr::ChangeCell(..)
			| BrainIr::SetCell(..)
			| BrainIr::InputIntoCell
			| BrainIr::OutputChar(..)
			| BrainIr::OutputCurrentCell
			| BrainIr::MoveValueTo(..)
			| BrainIr::FetchValueFrom(..) => {}
			_ => return None,
		}
	}
	Some(sum)
}
