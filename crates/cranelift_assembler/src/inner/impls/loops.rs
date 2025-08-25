use std::slice;

use cranelift_codegen::ir::InstBuilder as _;
use frick_assembler::AssemblyError;
use frick_ir::BrainIr;

use crate::{
	CraneliftAssemblyError,
	inner::{InnerAssembler, SrcLoc},
};

impl InnerAssembler<'_> {
	pub fn block(&mut self, ops: &[BrainIr]) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		self.invalidate_loads();

		self.add_srcflag(SrcLoc::BLOCK);

		let body_block = self.create_block();
		let next_block = self.create_block();

		let value = self.load(0);

		self.ins().brif(value, body_block, &[], next_block, &[]);

		self.switch_to_block(body_block);

		for op in ops {
			self.invalidate_loads();
			self.ops(slice::from_ref(op))?;
			self.add_srcflag(SrcLoc::BLOCK);
		}

		self.ins().jump(next_block, &[]);

		self.switch_to_block(next_block);
		self.seal_block(next_block);

		self.remove_srcflag(SrcLoc::BLOCK);

		Ok(())
	}

	pub fn dynamic_loop(
		&mut self,
		ops: &[BrainIr],
	) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		self.invalidate_loads();

		self.add_srcflag(SrcLoc::DYNAMIC_LOOP);

		let head_block = self.create_block();
		let body_block = self.create_block();
		let next_block = self.create_block();

		self.ins().jump(head_block, &[]);

		self.switch_to_block(head_block);

		let value = self.load(0);

		self.ins().brif(value, body_block, &[], next_block, &[]);

		self.switch_to_block(body_block);

		for op in ops {
			self.invalidate_loads();
			self.ops(slice::from_ref(op))?;
			self.add_srcflag(SrcLoc::DYNAMIC_LOOP);
		}

		self.ins().jump(head_block, &[]);

		self.switch_to_block(next_block);

		self.remove_srcflag(SrcLoc::DYNAMIC_LOOP);

		Ok(())
	}

	pub fn find_zero(&mut self, offset: i32) {
		self.invalidate_loads();

		self.add_srcflag(SrcLoc::FIND_ZERO);

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

		self.remove_srcflag(SrcLoc::FIND_ZERO);
	}
}
