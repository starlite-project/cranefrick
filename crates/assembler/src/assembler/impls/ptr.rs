use cranelift_codegen::ir::InstBuilder as _;

use crate::assembler::{Assembler, srclocs};

impl Assembler<'_> {
	pub fn move_pointer(&mut self, offset: i32) {
		self.invalidate_loads();

		self.add_srcflag(srclocs::MOVE_POINTER);

		let ptr_type = self.ptr_type;
		let memory_address = self.memory_address;

		let value = self.ins().iconst(ptr_type, i64::from(offset));
		self.memory_address = self.ins().iadd(memory_address, value);

		self.remove_srcflag(srclocs::MOVE_POINTER);
	}
}
