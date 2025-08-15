use cranelift_codegen::ir::{InstBuilder as _, types};

use crate::assembler::Assembler;

impl Assembler<'_> {
	pub fn output_char(&mut self, c: u8) {
		let write = self.write;

		let value = self.ins().iconst(types::I8, i64::from(c));

		self.ins().call(write, &[value]);
	}

	pub fn output_current_cell(&mut self) {
		let write = self.write;

		let value = self.load(0);
		self.ins().call(write, &[value]);
	}

	pub fn input_into_cell(&mut self) {
		self.invalidate_load();

		let read = self.read;
		let memory_address = self.memory_address;

		self.ins().call(read, &[memory_address]);
	}
}
