use cranelift_codegen::ir::InstBuilder as _;
use frick_ir::MoveOptions;

use crate::inner::{InnerAssembler, SrcLoc};

impl InnerAssembler<'_> {
	pub fn move_value_to(&mut self, options: MoveOptions) {
		self.invalidate_loads_at([0, options.offset()]);

		self.add_srcflag(SrcLoc::MOVE_VALUE);

		let current_value = self.load(0);
		self.set_cell(0, 0);

		let other_cell = self.load(options.offset());

		let value_to_add = self
			.ins()
			.imul_imm(current_value, i64::from(*options.factor()));

		let added = self.ins().iadd(other_cell, value_to_add);

		self.store(added, options.offset());

		self.remove_srcflag(SrcLoc::MOVE_VALUE);
	}

	pub fn take_value_to(&mut self, options: MoveOptions) {
		self.invalidate_loads_at([0, options.offset()]);

		self.add_srcflag(SrcLoc::TAKE_VALUE);

		let current_value = self.load(0);
		self.set_cell(0, 0);

		self.move_pointer(options.offset());

		let other_cell = self.load(0);

		let value_to_add = self
			.ins()
			.imul_imm(current_value, i64::from(*options.factor()));

		let added = self.ins().iadd(other_cell, value_to_add);

		self.store(added, 0);

		self.remove_srcflag(SrcLoc::TAKE_VALUE);
	}

	pub fn fetch_value_from(&mut self, options: MoveOptions) {
		self.invalidate_loads_at([0, options.offset()]);

		self.add_srcflag(SrcLoc::FETCH_VALUE);

		let other_cell = self.load(options.offset());

		self.set_cell(0, options.offset());

		let current_cell = self.load(0);

		let value_to_add = self
			.ins()
			.imul_imm(other_cell, i64::from(*options.factor()));

		let added = self.ins().iadd(current_cell, value_to_add);

		self.store(added, 0);

		self.remove_srcflag(SrcLoc::FETCH_VALUE);
	}

	pub fn replace_value_from(&mut self, options: MoveOptions) {
		self.invalidate_loads_at([0, options.offset()]);

		self.add_srcflag(SrcLoc::REPLACE_VALUE);

		let other_cell = self.load(options.offset());
		self.set_cell(0, options.offset());

		let value_to_store = self
			.ins()
			.imul_imm(other_cell, i64::from(*options.factor()));

		self.store(value_to_store, 0);

		self.remove_srcflag(SrcLoc::REPLACE_VALUE);
	}

	pub fn scale_value(&mut self, factor: u8) {
		self.invalidate_load_at(0);

		self.add_srcflag(SrcLoc::SCALE_VALUE);

		let cell = self.load(0);

		let value_to_store = self.ins().imul_imm(cell, i64::from(factor));

		self.store(value_to_store, 0);

		self.remove_srcflag(SrcLoc::SCALE_VALUE);
	}
}
