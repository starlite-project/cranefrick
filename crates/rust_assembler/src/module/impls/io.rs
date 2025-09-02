use frick_assembler::{frick_assembler_read, frick_assembler_write};

use crate::RustInterpreterModule;

impl RustInterpreterModule<'_> {
	pub(crate) fn output_current_cell(memory: &[u8; 30_000], current_ptr: usize) {
		let value = memory[current_ptr];

		let extended = value.into();

		unsafe {
			frick_assembler_write(extended);
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

	pub(crate) fn input_into_cell(memory: &mut [u8; 30_000], current_ptr: usize) {
		unsafe {
			frick_assembler_read(memory.as_mut_ptr().add(current_ptr));
		}
	}
}
