use std::mem;

use frick_assembler::TAPE_SIZE;

use crate::RustInterpreterModule;

impl RustInterpreterModule<'_> {
	pub(crate) const fn set_cell(
		value: u8,
		offset: i32,
		memory: &mut [u8; TAPE_SIZE],
		current_ptr: usize,
	) {
		let offset_ptr = Self::offset_ptr(current_ptr, offset);

		memory[offset_ptr] = value;
	}

	pub(crate) const fn change_cell(
		value: i8,
		offset: i32,
		memory: &mut [u8; TAPE_SIZE],
		current_ptr: usize,
	) {
		let offset_ptr = Self::offset_ptr(current_ptr, offset);

		memory[offset_ptr] = memory[offset_ptr].wrapping_add_signed(value);
	}

	pub(crate) fn sub_cell(offset: i32, memory: &mut [u8; TAPE_SIZE], current_ptr: usize) {
		let offset_ptr = Self::offset_ptr(current_ptr, offset);

		let current_value = mem::take(&mut memory[current_ptr]);

		memory[offset_ptr] = memory[offset_ptr].wrapping_sub(current_value);
	}
}
