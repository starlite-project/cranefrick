use crate::BrainMlir;

pub fn calculate_ptr_movement(ops: &[BrainMlir]) -> Option<i64> {
	let mut sum = 0i64;

	for op in ops {
		match op {
			BrainMlir::MovePtr(offset) => sum = sum.wrapping_add(*offset),
			BrainMlir::DynamicLoop(l) => {
				let loop_sum = calculate_ptr_movement(l)?;

				sum = sum.wrapping_add(loop_sum);
			}
			_ => {}
		}
	}
	Some(sum)
}
