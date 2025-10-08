use cranelift_codegen::ir::{InstBuilder as _, types};
use frick_assembler::AssemblyError;
use frick_ir::{BrainIr, OutputOptions};

use crate::{CraneliftAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn output(
		&mut self,
		options: &OutputOptions,
	) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		match options {
			OutputOptions::Cell(options) => {
				self.output_current_cell(options.value(), options.offset());
			}
			OutputOptions::Char(c) => self.output_char(*c),
			OutputOptions::Str(s) => self.output_chars(s),
			_ => {
				return Err(AssemblyError::NotImplemented(BrainIr::Output(
					options.clone(),
				)));
			}
		}

		Ok(())
	}

	fn output_char(&mut self, c: u8) {
		let write = self.write;

		let value = self.ins().iconst(types::I32, i64::from(c));

		self.ins().call(write, &[value]);
	}

	pub fn output_chars(&mut self, chars: &[u8]) {
		let write = self.write;

		for c in chars.iter().copied() {
			let value = self.ins().iconst(types::I32, i64::from(c));

			self.ins().call(write, &[value]);
		}
	}

	pub fn output_current_cell(&mut self, cell_offset: i8, offset: i32) {
		let write = self.write;

		let value = self.load(offset);

		let value = self.ins().sextend(types::I32, value);

		let value = self.ins().iadd_imm(value, i64::from(cell_offset));

		self.ins().call(write, &[value]);
	}

	pub fn input_into_cell(&mut self) {
		self.invalidate_load_at(0);

		let read = self.read;
		let value = self.ptr_value();

		self.ins().call(read, &[value]);
	}
}
