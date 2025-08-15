use cranelift_codegen::ir::{InstBuilder as _, types};

use crate::assembler::{Assembler, srclocs};

impl Assembler<'_> {
	pub fn output_char(&mut self, c: u8) {
		self.add_srcflag(srclocs::OUTPUT_CHAR);

		let write = self.write;

		let value = self.ins().iconst(types::I8, i64::from(c));

		self.ins().call(write, &[value]);

		self.remove_srcflag(srclocs::OUTPUT_CHAR);
	}

	pub fn output_current_cell(&mut self) {
		self.add_srcflag(srclocs::OUTPUT_CURRENT_CELL);

		let write = self.write;

		let value = self.load(0);
		self.ins().call(write, &[value]);

		self.remove_srcflag(srclocs::OUTPUT_CURRENT_CELL);
	}

	pub fn input_into_cell(&mut self) {
		self.invalidate_loads();

		self.add_srcflag(srclocs::INPUT_INTO_CELL);

		let read = self.read;
		let memory_address = self.memory_address;

		self.ins().call(read, &[memory_address]);

		self.remove_srcflag(srclocs::INPUT_INTO_CELL);
	}
}
