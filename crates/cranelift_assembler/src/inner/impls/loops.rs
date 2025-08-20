use std::slice;

use cranelift_codegen::ir::InstBuilder as _;
use frick_assembler::AssemblyError;
use frick_ir::BrainIr;

use crate::{
	CraneliftAssemblyError,
	inner::{InnerAssembler, SrcLoc},
};

impl InnerAssembler<'_> {
	pub fn if_nz(&mut self, ops: &[BrainIr]) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		self.add_srcflag(SrcLoc::IF_NZ);

		let body_block = self.create_block();
		let next_block = self.create_block();

		let value = self.load(0);

		self.ins().brif(value, body_block, &[], next_block, &[]);

		self.switch_to_block(body_block);

		for op in ops {
			self.ops(slice::from_ref(op))?;
			self.add_srcflag(SrcLoc::IF_NZ);
		}

		self.ins().jump(next_block, &[]);

		self.switch_to_block(next_block);
		self.seal_block(next_block);

		self.remove_srcflag(SrcLoc::IF_NZ);

		Ok(())
	}

	pub fn dynamic_loop(
		&mut self,
		ops: &[BrainIr],
	) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		println!("{}", ops.len());

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
			self.ops(slice::from_ref(op))?;
			self.add_srcflag(SrcLoc::DYNAMIC_LOOP);
		}

		self.ins().jump(head_block, &[]);

		self.switch_to_block(next_block);

		self.remove_srcflag(SrcLoc::DYNAMIC_LOOP);

		Ok(())
	}

	pub fn find_zero(&mut self, offset: i32) {
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
