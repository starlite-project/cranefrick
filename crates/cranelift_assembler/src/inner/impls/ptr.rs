use cranelift_codegen::ir::InstBuilder as _;

use crate::inner::{InnerAssembler, SrcLoc};

impl InnerAssembler<'_> {
	pub fn move_pointer(&mut self, offset: i32) {
		self.add_srcflag(SrcLoc::MOVE_POINTER);

		self.ptr_value = self.calculate_ptr(offset);
		self.remove_srcflag(SrcLoc::MOVE_POINTER);

		// self.ptr_value = self.ptr_value.wrapping_add(offset);
	}

	pub const fn calculate_ptr(&self, offset: i32) -> i32 {
		const LEN: i32 = 30_000;
		let ptr = self.ptr_value;

		let n = LEN + offset % LEN;

		(ptr + n) % LEN
	}
}
