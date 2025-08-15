use cranefrick_mlir::BrainMlir;
use cranelift_codegen::ir::InstBuilder as _;

use crate::{AssemblyError, assembler::Assembler};

impl Assembler<'_> {
	pub fn if_nz(&mut self, ops: &[BrainMlir]) -> Result<(), AssemblyError> {
		self.invalidate_loads();

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
		self.ops(ops)?;

		let memory_address = self.memory_address;
		self.ins().jump(next_block, &[memory_address.into()]);

		self.switch_to_block(next_block);
		self.seal_block(next_block);
		self.memory_address = self.block_params(next_block)[0];

		self.set_cell(0, 0);

		Ok(())
	}

	pub fn dynamic_loop(&mut self, ops: &[BrainMlir]) -> Result<(), AssemblyError> {
		self.invalidate_loads();

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
		self.ops(ops)?;

		let memory_address = self.memory_address;
		self.ins().jump(head_block, &[memory_address.into()]);

		self.switch_to_block(next_block);
		self.memory_address = self.block_params(next_block)[0];

		self.set_cell(0, 0);

		Ok(())
	}

	pub fn find_zero(&mut self, offset: i32) {
		self.invalidate_loads();

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
	}
}
