use std::slice;

use cranelift_codegen::ir::InstBuilder as _;
use frick_assembler::AssemblyError;
use frick_ir::BrainIr;

use crate::{CraneliftAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn if_not_zero(
		&mut self,
		ops: &[BrainIr],
		op_count: u32,
	) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		self.invalidate_loads();

		let body_block = self.create_block();
		let next_block = self.create_block();

		let value = self.load(0);

		self.ins().brif(value, body_block, &[], next_block, &[]);

		self.switch_to_block(body_block);

		for (i, op) in ops.iter().enumerate() {
			self.invalidate_loads();
			self.ops(slice::from_ref(op), op_count + i as u32)?;
		}

		self.ins().jump(next_block, &[]);

		self.switch_to_block(next_block);
		self.seal_block(next_block);

		Ok(())
	}

	pub fn dynamic_loop(
		&mut self,
		ops: &[BrainIr],
		op_count: u32,
	) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		self.invalidate_loads();

		let head_block = self.create_block();
		let body_block = self.create_block();
		let next_block = self.create_block();

		self.ins().jump(head_block, &[]);

		self.switch_to_block(head_block);

		let value = self.load(0);

		self.ins().brif(value, body_block, &[], next_block, &[]);

		self.switch_to_block(body_block);

		for (i, op) in ops.iter().enumerate() {
			self.invalidate_loads();
			self.ops(slice::from_ref(op), op_count + i as u32)?;
		}

		self.ins().jump(head_block, &[]);

		self.switch_to_block(next_block);

		Ok(())
	}

	pub fn find_zero(&mut self, offset: i32) {
		self.invalidate_loads();

		let head_block = self.create_block();
		let body_block = self.create_block();
		let next_block = self.create_block();

		self.ins().jump(head_block, &[]);

		self.switch_to_block(head_block);

		let value = self.load(0);

		self.ins().brif(value, body_block, &[], next_block, &[]);

		self.switch_to_block(body_block);

		self.move_pointer(offset);

		self.ins().jump(head_block, &[]);

		self.switch_to_block(next_block);
	}
}
