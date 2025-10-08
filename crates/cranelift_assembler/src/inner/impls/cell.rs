use cranelift_codegen::ir::InstBuilder as _;

use crate::inner::InnerAssembler;

impl InnerAssembler<'_> {
	pub fn set_cell(&mut self, value: u8, offset: i32) {
		self.store_value(value, offset);
	}

	pub fn change_cell(&mut self, value: i8, offset: i32) {
		let current_cell_value = self.load(offset);

		let added = self.ins().iadd_imm(current_cell_value, i64::from(value));

		self.store(added, offset);
	}
}
