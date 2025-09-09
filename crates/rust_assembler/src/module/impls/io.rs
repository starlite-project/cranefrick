use frick_assembler::{TAPE_SIZE, frick_assembler_read, frick_assembler_write};

use crate::RustInterpreterModule;

impl RustInterpreterModule<'_> {
	pub(crate) fn output_current_cell(
		cell_offset: i8,
		offset: i32,
		memory: &[u8; TAPE_SIZE],
		current_ptr: usize,
	) {
		let offset_ptr = Self::offset_ptr(current_ptr, offset);

		let value = memory[offset_ptr];

		let extended = u32::from(value);

		let output = extended.wrapping_add_signed(cell_offset.into());

		unsafe {
			frick_assembler_write(output);
		}
	}

	pub(crate) fn output_char(c: u8) {
		let extended = c.into();

		unsafe {
			frick_assembler_write(extended);
		}
	}

	pub(crate) fn output_chars(c: &[u8]) {
		c.iter().copied().for_each(Self::output_char);
	}

	pub(crate) fn input_into_cell(memory: &mut [u8; TAPE_SIZE], current_ptr: usize) {
		unsafe {
			frick_assembler_read(memory.as_mut_ptr().add(current_ptr));
		}
	}
}
