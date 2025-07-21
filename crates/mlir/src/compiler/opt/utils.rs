use crate::BrainMlir;

pub fn calculate_ptr_movement(ops: &[BrainMlir]) -> Option<i32> {
	let mut sum = 0i32;

	for op in ops {
		match op {
			BrainMlir::MovePointer(offset) => sum = sum.wrapping_add(*offset),
			BrainMlir::DynamicLoop(l) => {
				let loop_sum = calculate_ptr_movement(l)?;

				sum = sum.wrapping_add(loop_sum);
			}
			_ => {}
		}
	}
	Some(sum)
}
