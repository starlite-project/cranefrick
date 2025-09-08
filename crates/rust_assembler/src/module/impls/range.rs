use std::ops::RangeInclusive;

use frick_assembler::TAPE_SIZE;

use crate::RustInterpreterModule;

impl RustInterpreterModule<'_> {
	pub(crate) fn mem_set(
		value: u8,
		range: RangeInclusive<i32>,
		memory: &mut [u8; TAPE_SIZE],
		current_ptr: usize,
	) {
		for i in range {
			Self::set_cell(value, i, memory, current_ptr);
		}
	}
}
