use cranelift_codegen::ir::InstBuilder as _;

use crate::assembler::{Assembler, srclocs};

impl Assembler<'_> {
	pub fn move_value(&mut self, factor: u8, offset: i32) {
		self.add_srcflag(srclocs::MOVE_VALUE);

		let current_value = self.load(0);
		self.set_cell(0, 0);

		let other_cell = self.load(offset);

		let value_to_add = self.ins().imul_imm(current_value, i64::from(factor));

		let added = self.ins().iadd(other_cell, value_to_add);

		self.store(added, offset);

		self.remove_srcflag(srclocs::MOVE_VALUE);
	}

	pub fn take_value(&mut self, factor: u8, offset: i32) {
		self.add_srcflag(srclocs::TAKE_VALUE);

		let current_value = self.load(0);
		self.set_cell(0, 0);

		self.move_pointer(offset);

		let other_cell = self.load(0);

		let value_to_add = self.ins().imul_imm(current_value, i64::from(factor));

		let added = self.ins().iadd(other_cell, value_to_add);

		self.store(added, 0);

		self.remove_srcflag(srclocs::TAKE_VALUE);
	}

	pub fn fetch_value(&mut self, factor: u8, offset: i32) {
		self.add_srcflag(srclocs::FETCH_VALUE);

		let other_cell = self.load(offset);

		self.set_cell(0, offset);

		let current_cell = self.load(0);

		let value_to_add = self.ins().imul_imm(other_cell, i64::from(factor));

		let added = self.ins().iadd(current_cell, value_to_add);

		self.store(added, 0);

		self.remove_srcflag(srclocs::FETCH_VALUE);
	}
}
