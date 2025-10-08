use cranelift_codegen::ir::{InstBuilder as _, Value, condcodes::IntCC, types};
use frick_assembler::TAPE_SIZE;

use crate::inner::InnerAssembler;

impl InnerAssembler<'_> {
	pub fn move_pointer(&mut self, offset: i32) {
		let ptr_var = self.ptr;

		let wrapped_pointer = self.offset_pointer(offset);

		self.def_var(ptr_var, wrapped_pointer);
	}

	pub fn load_pointer(&mut self) -> Value {
		let ptr_var = self.ptr;

		self.use_var(ptr_var)
	}

	pub fn offset_pointer(&mut self, offset: i32) -> Value {
		let current_pointer = self.load_pointer();

		if matches!(offset, 0) {
			current_pointer
		} else {
			let offset_pointer = self.ins().iconst(types::I64, i64::from(offset));

			if offset > 0 {
				self.wrap_pointer_positive(offset_pointer)
			} else {
				self.wrap_pointer_negative(offset_pointer)
			}
		}
	}

	fn wrap_pointer_positive(&mut self, offset_pointer: Value) -> Value {
		self.ins().urem_imm(offset_pointer, TAPE_SIZE as i64)
	}

	fn wrap_pointer_negative(&mut self, offset_pointer: Value) -> Value {
		let tmp = self.ins().srem_imm(offset_pointer, TAPE_SIZE as i64);

		let added_offset = self.ins().iadd_imm(tmp, TAPE_SIZE as i64);

		let cmp = self.ins().icmp_imm(IntCC::SignedLessThan, tmp, 0);

		self.ins().select(cmp, added_offset, tmp)
	}
}
