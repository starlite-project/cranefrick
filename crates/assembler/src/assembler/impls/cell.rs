use cranelift_codegen::ir::{InstBuilder as _, types};

use crate::assembler::{Assembler, srclocs};

impl Assembler<'_> {
	pub fn change_cell(&mut self, value: i8, offset: i32) {
		self.invalidate_loads();

		self.add_srcflag(srclocs::CHANGE_CELL);

		let heap_value = self.load(offset);
		let changed = if value.is_negative() {
			let sub_value = self
				.ins()
				.iconst(types::I8, i64::from(value.unsigned_abs()));
			self.ins().isub(heap_value, sub_value)
		} else {
			self.ins().iadd_imm(heap_value, i64::from(value))
		};

		self.store(changed, offset);

		self.remove_srcflag(srclocs::CHANGE_CELL);
	}

	pub fn set_cell(&mut self, value: u8, offset: i32) {
		self.invalidate_loads();

		self.add_srcflag(srclocs::SET_CELL);

		let new_value = self.const_u8(value);
		self.store(new_value, offset);

		self.remove_srcflag(srclocs::SET_CELL);
	}
}
