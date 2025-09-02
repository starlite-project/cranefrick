use frick_assembler::TAPE_SIZE;
use frick_ir::BrainIr;

use crate::RustInterpreterModule;

impl RustInterpreterModule<'_> {
	pub(crate) fn if_not_zero(
		ops: &[BrainIr],
		memory: &mut [u8; TAPE_SIZE],
		current_ptr: &mut usize,
	) {
		let current_value = memory[*current_ptr];

		if !matches!(current_value, 0) {
			for op in ops {
				Self::execute_op(op, memory, current_ptr);
			}
		}
	}

	pub(crate) fn dynamic_loop(
		ops: &[BrainIr],
		memory: &mut [u8; TAPE_SIZE],
		current_ptr: &mut usize,
	) {
		loop {
			match memory[*current_ptr] {
				0 => break,
				_ => {
					for op in ops {
						Self::execute_op(op, memory, current_ptr);
					}
				}
			}
		}
	}

	pub(crate) const fn find_zero(offset: i32, memory: &[u8; TAPE_SIZE], current_ptr: &mut usize) {
		while !matches!(memory[*current_ptr], 0) {
			Self::move_pointer(offset, current_ptr);
		}
	}
}
