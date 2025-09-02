use frick_assembler::TAPE_SIZE;

use crate::RustInterpreterModule;

impl RustInterpreterModule<'_> {
	pub(crate) const fn move_pointer(offset: i32, current_ptr: &mut usize) {
		*current_ptr = Self::offset_ptr(*current_ptr, offset);
	}

	pub(crate) const fn offset_ptr(current_ptr: usize, offset: i32) -> usize {
		if matches!(offset, 0) {
			return current_ptr;
		}

		if offset > 0 {
			(current_ptr + offset as usize) % TAPE_SIZE
		} else {
			(current_ptr + TAPE_SIZE - offset.unsigned_abs() as usize) % TAPE_SIZE
		}
	}
}
