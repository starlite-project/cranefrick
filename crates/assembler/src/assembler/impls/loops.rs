use std::slice;

use cranefrick_ir::BrainIr;
use cranelift_codegen::ir::InstBuilder as _;

use crate::{
	AssemblyError,
	assembler::{Assembler, srclocs},
};

impl Assembler<'_> {
	pub fn if_nz(&mut self, ops: &[BrainIr]) -> Result<(), AssemblyError> {
		// self.invalidate_load_at(0);
		self.invalidate_loads();

		self.add_srcflag(srclocs::IF_NZ);

		let ptr_type = self.ptr_type;
		let memory_address = self.memory_address;

		let body_block = self.create_block();
		let next_block = self.create_block();

		self.append_block_param(body_block, ptr_type);
		self.append_block_param(next_block, ptr_type);

		let value = self.load(0);

		self.ins().brif(
			value,
			body_block,
			&[memory_address.into()],
			next_block,
			&[memory_address.into()],
		);

		self.switch_to_block(body_block);
		for op in ops {
			self.invalidate_loads();
			self.ops(slice::from_ref(op))?;
			self.add_srcflag(srclocs::IF_NZ);
		}

		let memory_address = self.memory_address;
		self.ins().jump(next_block, &[memory_address.into()]);

		self.switch_to_block(next_block);
		self.seal_block(next_block);
		self.memory_address = self.block_params(next_block)[0];

		self.remove_srcflag(srclocs::IF_NZ);

		Ok(())
	}

	pub fn dynamic_loop(&mut self, ops: &[BrainIr]) -> Result<(), AssemblyError> {
		self.invalidate_loads();

		self.add_srcflag(srclocs::DYNAMIC_LOOP);

		let ptr_type = self.ptr_type;
		let memory_address = self.memory_address;

		let head_block = self.create_block();
		let body_block = self.create_block();
		let next_block = self.create_block();

		self.append_block_param(head_block, ptr_type);
		self.append_block_param(body_block, ptr_type);
		self.append_block_param(next_block, ptr_type);

		self.ins().jump(head_block, &[memory_address.into()]);

		self.switch_to_block(head_block);
		self.memory_address = self.block_params(head_block)[0];

		let value = self.load(0);
		let memory_address = self.memory_address;

		self.ins().brif(
			value,
			body_block,
			&[memory_address.into()],
			next_block,
			&[memory_address.into()],
		);

		self.switch_to_block(body_block);

		for op in ops {
			self.invalidate_loads();
			self.ops(slice::from_ref(op))?;
			self.add_srcflag(srclocs::DYNAMIC_LOOP);
		}

		let memory_address = self.memory_address;
		self.ins().jump(head_block, &[memory_address.into()]);

		self.switch_to_block(next_block);
		self.memory_address = self.block_params(next_block)[0];

		self.remove_srcflag(srclocs::DYNAMIC_LOOP);

		Ok(())
	}

	pub fn find_zero(&mut self, offset: i32) {
		self.invalidate_loads();

		self.add_srcflag(srclocs::FIND_ZERO);

		let ptr_type = self.ptr_type;
		let memory_address = self.memory_address;

		let head_block = self.create_block();
		let body_block = self.create_block();
		let next_block = self.create_block();

		self.append_block_param(head_block, ptr_type);
		self.append_block_param(body_block, ptr_type);
		self.append_block_param(next_block, ptr_type);

		self.ins().jump(head_block, &[memory_address.into()]);

		self.switch_to_block(head_block);
		let memory_address = self.block_params(head_block)[0];
		self.memory_address = memory_address;

		let value = self.load(0);

		self.ins().brif(
			value,
			body_block,
			&[memory_address.into()],
			next_block,
			&[memory_address.into()],
		);

		self.switch_to_block(body_block);
		self.memory_address = self.block_params(body_block)[0];

		self.move_pointer(offset);
		let memory_address = self.memory_address;

		self.ins().jump(head_block, &[memory_address.into()]);

		self.switch_to_block(next_block);
		self.memory_address = self.block_params(next_block)[0];

		self.remove_srcflag(srclocs::FIND_ZERO);
	}
}
