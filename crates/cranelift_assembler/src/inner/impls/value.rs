use cranelift_codegen::ir::InstBuilder as _;
use frick_ir::CellChangeOptions;

use crate::inner::InnerAssembler;

impl InnerAssembler<'_> {
	pub fn move_value_to(&mut self, options: CellChangeOptions) {
		self.invalidate_loads_at([0, options.offset()]);

		let current_value = self.load(0);
		self.set_cell(0, 0);

		let other_cell = self.load(options.offset());

		let value_to_add = self
			.ins()
			.imul_imm(current_value, i64::from(options.value()));

		let added = self.ins().iadd(other_cell, value_to_add);

		self.store(added, options.offset());
	}

	pub fn take_value_to(&mut self, options: CellChangeOptions) {
		self.invalidate_loads_at([0, options.offset()]);

		let current_value = self.load(0);
		self.set_cell(0, 0);

		self.move_pointer(options.offset());

		let other_cell = self.load(0);

		let value_to_add = self
			.ins()
			.imul_imm(current_value, i64::from(options.value()));

		let added = self.ins().iadd(other_cell, value_to_add);

		self.store(added, 0);
	}

	pub fn fetch_value_from(&mut self, options: CellChangeOptions) {
		self.invalidate_loads_at([0, options.offset()]);

		let other_cell = self.load(options.offset());

		self.set_cell(0, options.offset());

		let current_cell = self.load(0);

		let value_to_add = self.ins().imul_imm(other_cell, i64::from(options.value()));

		let added = self.ins().iadd(current_cell, value_to_add);

		self.store(added, 0);
	}

	pub fn replace_value_from(&mut self, options: CellChangeOptions) {
		self.invalidate_loads_at([0, options.offset()]);

		let other_cell = self.load(options.offset());
		self.set_cell(0, options.offset());

		let value_to_store = self.ins().imul_imm(other_cell, i64::from(options.value()));

		self.store(value_to_store, 0);
	}

	pub fn scale_value(&mut self, factor: u8) {
		self.invalidate_load_at(0);

		let cell = self.load(0);

		let value_to_store = self.ins().imul_imm(cell, i64::from(factor));

		self.store(value_to_store, 0);
	}
}
