use std::mem;

use frick_assembler::TAPE_SIZE;

use crate::RustInterpreterModule;

impl RustInterpreterModule<'_> {
	pub(crate) fn move_value_to(
		factor: u8,
		offset: i32,
		memory: &mut [u8; TAPE_SIZE],
		current_ptr: usize,
	) {
		let offset_ptr = Self::offset_ptr(current_ptr, offset);

		let value = mem::take(&mut memory[current_ptr]);

		let multiplied = value.wrapping_mul(factor);

		memory[offset_ptr] = memory[offset_ptr].wrapping_add(multiplied);
	}

	pub(crate) fn take_value_to(
		factor: u8,
		offset: i32,
		memory: &mut [u8; TAPE_SIZE],
		current_ptr: &mut usize,
	) {
		Self::move_value_to(factor, offset, memory, *current_ptr);
		Self::move_pointer(offset, current_ptr);
	}

	pub(crate) fn fetch_value_from(
		factor: u8,
		offset: i32,
		memory: &mut [u8; TAPE_SIZE],
		current_ptr: usize,
	) {
		let offset_ptr = Self::offset_ptr(current_ptr, offset);

		let value = mem::take(&mut memory[offset_ptr]);

		let multiplied = value.wrapping_mul(factor);

		memory[current_ptr] = memory[current_ptr].wrapping_add(multiplied);
	}

	pub(crate) fn replace_value_from(
		factor: u8,
		offset: i32,
		memory: &mut [u8; TAPE_SIZE],
		current_ptr: usize,
	) {
		Self::set_cell(0, 0, memory, current_ptr);
		Self::fetch_value_from(factor, offset, memory, current_ptr);
	}

	pub(crate) const fn scale_value(factor: u8, memory: &mut [u8; TAPE_SIZE], current_ptr: usize) {
		memory[current_ptr] = memory[current_ptr].wrapping_mul(factor);
	}
}
