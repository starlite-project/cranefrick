use cranelift_codegen::ir::InstBuilder as _;

use crate::assembler::{Assembler, srclocs};

impl Assembler<'_> {
	pub fn move_pointer(&mut self, offset: i32) {
		self.add_srcflag(srclocs::MOVE_POINTER);

		let ptr_type = self.ptr_type;
		let memory_address = self.memory_address;

		let value = self.ins().iconst(ptr_type, i64::from(offset));
		self.memory_address = self.ins().iadd(memory_address, value);

		// self.tape_idx = (self.tape_idx.wrapping_add_signed(offset as isize)) % 30_000;

		let old_tape_idx = self.tape_idx;

		if offset.is_positive() {
			self.tape_idx = (old_tape_idx + offset as usize) % 30_000;
		} else {
			self.tape_idx = ((self.tape_idx + 30_000).wrapping_sub(offset as usize)) % 30_000;
		}

		self.remove_srcflag(srclocs::MOVE_POINTER);
	}
}
