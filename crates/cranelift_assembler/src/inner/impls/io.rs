use cranelift_codegen::ir::{InstBuilder as _, types};
use frick_assembler::AssemblyError;
use frick_ir::{BrainIr, CellChangeOptions, OutputOptions};

use crate::{CraneliftAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn output(
		&mut self,
		options: &OutputOptions,
	) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		match options {
			OutputOptions::Cell(options) => self.output_cell(*options),
			OutputOptions::Cells(options) => self.output_cells(options),
			OutputOptions::Char(c) => self.output_char(*c),
			_ => {
				return Err(AssemblyError::NotImplemented(BrainIr::Output(
					options.clone(),
				)));
			}
		}

		Ok(())
	}

	fn output_cells(&mut self, options: &[CellChangeOptions<i8>]) {
		options
			.iter()
			.copied()
			.for_each(|opt| self.output_cell(opt));
	}

	fn output_cell(&mut self, options: CellChangeOptions<i8>) {
		let putchar = self.putchar;

		let current_cell = self.load(options.offset());

		let offset_cell_value = if matches!(options.value(), 0) {
			current_cell
		} else {
			self.ins()
				.iadd_imm(current_cell, i64::from(options.value()))
		};

		let extended_cell_value = self.ins().uextend(types::I32, offset_cell_value);

		self.ins().call(putchar, &[extended_cell_value]);
	}

	fn output_char(&mut self, c: u8) {
		let putchar = self.putchar;

		let char_value = self.ins().iconst(types::I32, i64::from(c));

		self.ins().call(putchar, &[char_value]);
	}
}
