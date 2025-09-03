use cranelift_codegen::ir::{InstBuilder as _, types};

use crate::inner::{InnerAssembler, SrcLoc};

impl InnerAssembler<'_> {
	pub fn output_char(&mut self, c: u8) {
		self.add_srcflag(SrcLoc::OUTPUT_CHAR);

		let write = self.write;

		let value = self.ins().iconst(types::I32, i64::from(c));

		self.ins().call(write, &[value]);

		self.remove_srcflag(SrcLoc::OUTPUT_CHAR);
	}

	pub fn output_chars(&mut self, chars: &[u8]) {
		self.add_srcflag(SrcLoc::OUTPUT_CHARS);

		let write = self.write;

		for c in chars.iter().copied() {
			let value = self.ins().iconst(types::I32, i64::from(c));

			self.ins().call(write, &[value]);
		}

		self.remove_srcflag(SrcLoc::OUTPUT_CHARS);
	}

	pub fn output_current_cell(&mut self) {
		self.add_srcflag(SrcLoc::OUTPUT_CURRENT_CELL);

		let write = self.write;

		let value = self.load(0);

		let value = self.ins().sextend(types::I32, value);

		self.ins().call(write, &[value]);

		self.remove_srcflag(SrcLoc::OUTPUT_CURRENT_CELL);
	}

	pub fn output_current_cell_offset_by(&mut self, cell_offset: i8) {
		self.add_srcflag(SrcLoc::OUTPUT_CURRENT_CELL_OFFSET_BY);

		let write = self.write;

		let value = self.load(0);

		let value = self.ins().sextend(types::I32, value);

		let added_value = self.ins().iadd_imm(value, i64::from(cell_offset));

		self.ins().call(write, &[added_value]);

		self.remove_srcflag(SrcLoc::OUTPUT_CURRENT_CELL_OFFSET_BY);
	}

	pub fn input_into_cell(&mut self) {
		self.invalidate_load_at(0);

		self.add_srcflag(SrcLoc::INPUT_INTO_CELL);

		let read = self.read;
		let value = self.ptr_value();

		self.ins().call(read, &[value]);

		self.remove_srcflag(SrcLoc::INPUT_INTO_CELL);
	}
}
