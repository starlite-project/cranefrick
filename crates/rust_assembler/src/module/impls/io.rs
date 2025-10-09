use frick_assembler::TAPE_SIZE;
use frick_ir::OutputOptions;

use crate::RustInterpreterModule;

impl RustInterpreterModule<'_> {
	pub(crate) fn output(options: &OutputOptions, memory: &[u8; TAPE_SIZE], current_ptr: usize) {
		match options {
			OutputOptions::Cell(options) => {
				Self::output_current_cell(options.value(), options.offset(), memory, current_ptr);
			}
			OutputOptions::Char(c) => Self::output_char(*c),
			OutputOptions::Str(s) => Self::output_chars(s),
			_ => unimplemented!(),
		}
	}

	fn output_current_cell(
		cell_offset: i8,
		offset: i32,
		memory: &[u8; TAPE_SIZE],
		current_ptr: usize,
	) {
		let offset_ptr = Self::offset_ptr(current_ptr, offset);

		let value = memory[offset_ptr];

		let extended = u32::from(value);

		let output = extended.wrapping_add_signed(cell_offset.into()) as i32;

		unsafe {
			let putchar_output = frick_interop::rust_putchar(output);
			assert_eq!(putchar_output, output);
		}
	}

	fn output_char(c: u8) {
		let extended = u32::from(c) as i32;

		unsafe {
			let output = frick_interop::rust_putchar(extended);
			assert_eq!(extended, output);
		}
	}

	fn output_chars(c: &[u8]) {
		c.iter().copied().for_each(Self::output_char);
	}

	pub(crate) fn input_into_cell(memory: &mut [u8; TAPE_SIZE], current_ptr: usize) {
		let value = unsafe { frick_interop::rust_getchar() };

		let trunc = value as u8;

		memory[current_ptr] = trunc;
	}
}
