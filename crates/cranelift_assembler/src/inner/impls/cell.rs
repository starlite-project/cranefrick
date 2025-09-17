use cranelift_codegen::ir::InstBuilder as _;

use crate::inner::{InnerAssembler, SrcLoc};

impl InnerAssembler<'_> {
	pub fn change_cell(&mut self, value: i8, offset: i32) {
		self.invalidate_load_at(offset);

		self.add_srcflag(SrcLoc::CHANGE_CELL);

		let heap_value = self.load(offset);

		let changed = self.ins().iadd_imm(heap_value, i64::from(value));

		self.store(changed, offset);

		self.remove_srcflag(SrcLoc::CHANGE_CELL);
	}

	pub fn set_cell(&mut self, value: u8, offset: i32) {
		self.invalidate_load_at(offset);

		self.add_srcflag(SrcLoc::SET_CELL);

		let new_value = self.const_u8(value);
		self.store(new_value, offset);

		self.remove_srcflag(SrcLoc::SET_CELL);
	}
}
