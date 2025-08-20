use cranelift_codegen::ir::InstBuilder as _;

use crate::inner::{InnerAssembler, SrcLoc};

impl InnerAssembler<'_> {
	pub fn output_char(&mut self, c: u8) {
		self.add_srcflag(SrcLoc::OUTPUT_CHAR);

		let write = self.write;

		let value = self.const_u8(c);

		self.ins().call(write, &[value]);

		self.remove_srcflag(SrcLoc::OUTPUT_CHAR);
	}

	pub fn output_chars(&mut self, chars: &[u8]) {
		self.add_srcflag(SrcLoc::OUTPUT_CHARS);

		let write = self.write;

		for c in chars.iter().copied() {
			let value = self.const_u8(c);

			self.ins().call(write, &[value]);
		}

		self.remove_srcflag(SrcLoc::OUTPUT_CHARS);
	}

	pub fn output_current_cell(&mut self) {
		self.add_srcflag(SrcLoc::OUTPUT_CURRENT_CELL);

		let write = self.write;

		let value = self.load(0);

		self.ins().call(write, &[value]);

		self.remove_srcflag(SrcLoc::OUTPUT_CURRENT_CELL);
	}

	pub fn input_into_cell(&mut self) {
		self.add_srcflag(SrcLoc::INPUT_INTO_CELL);

		let read = self.read;
		let value = self.load(0);

		self.ins().call(read, &[value]);

		self.remove_srcflag(SrcLoc::INPUT_INTO_CELL);
	}
}
