use cranelift_codegen::ir::{InstBuilder as _, types};

use crate::inner::{InnerAssembler, SrcLoc};

impl InnerAssembler<'_> {
	pub fn change_cell(&mut self, value: i8, offset: i32) {
		self.invalidate_load_at(offset);

		self.add_srcflag(SrcLoc::CHANGE_CELL);

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

		self.remove_srcflag(SrcLoc::CHANGE_CELL);
	}

	pub fn set_cell(&mut self, value: u8, offset: i32) {
		self.invalidate_load_at(offset);

		self.add_srcflag(SrcLoc::SET_CELL);

		let new_value = self.const_u8(value);
		self.store(new_value, offset);

		self.remove_srcflag(SrcLoc::SET_CELL);
	}

	pub fn sub_cell(&mut self, offset: i32) {
		self.invalidate_loads_at([0, offset]);

		self.add_srcflag(SrcLoc::SUB_CELL);

		let subtractor = self.load(0);

		self.set_cell(0, 0);

		let other_value = self.load(offset);

		let value_to_store = self.ins().isub(other_value, subtractor);

		self.store(value_to_store, offset);

		self.remove_srcflag(SrcLoc::SUB_CELL);
	}
}
