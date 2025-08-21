use frick_assembler::AssemblyError;
use frick_ir::BrainIr;
use inkwell::IntPredicate;

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn if_nz(&mut self, ops: &[BrainIr]) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		let body_block = self
			.context
			.append_basic_block(self.functions.main, "if_nz body");
		let next_block = self
			.context
			.append_basic_block(self.functions.main, "if_nz next");

		let value = self.load(0)?;

		let zero = {
			let i8_type = self.context.i8_type();

			i8_type.const_zero()
		};

		let cmp = self
			.builder
			.build_int_compare(IntPredicate::NE, value, zero, "check if value is zero")
			.map_err(AssemblyError::backend)?;

		self.builder
			.build_conditional_branch(cmp, body_block, next_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(body_block);

		self.ops(ops)?;

		self.builder
			.build_unconditional_branch(next_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(next_block);

		Ok(())
	}

	pub fn dynamic_loop(&mut self, ops: &[BrainIr]) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		let head_block = self
			.context
			.append_basic_block(self.functions.main, "dynamic_loop head");
		let body_block = self
			.context
			.append_basic_block(self.functions.main, "dynamic_loop body");
		let next_block = self
			.context
			.append_basic_block(self.functions.main, "dynamic_loop next");

		self.builder
			.build_unconditional_branch(head_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(head_block);

		let value = self.load(0)?;

		let zero = {
			let i8_type = self.context.i8_type();

			i8_type.const_zero()
		};

		let cmp = self
			.builder
			.build_int_compare(IntPredicate::NE, value, zero, "check if value is zero")
			.map_err(AssemblyError::backend)?;

		self.builder
			.build_conditional_branch(cmp, body_block, next_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(body_block);

		self.ops(ops)?;

		self.builder
			.build_unconditional_branch(head_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(next_block);

		Ok(())
	}

	pub fn find_zero(&mut self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let head_block = self
			.context
			.append_basic_block(self.functions.main, "find_zero head");
		let body_block = self
			.context
			.append_basic_block(self.functions.main, "find_zero body");
		let next_block = self
			.context
			.append_basic_block(self.functions.main, "find_zero next");

		self.builder.build_unconditional_branch(head_block)?;

		self.builder.position_at_end(head_block);

		let value = self.load(0)?;

		let zero = {
			let i8_type = self.context.i8_type();

			i8_type.const_zero()
		};

		let cmp = self.builder.build_int_compare(
			IntPredicate::NE,
			value,
			zero,
			"check if value is zero",
		)?;

		self.builder
			.build_conditional_branch(cmp, body_block, next_block)?;

		self.builder.position_at_end(body_block);

		self.move_pointer(offset)?;

		self.builder.build_unconditional_branch(head_block)?;

		self.builder.position_at_end(next_block);

		Ok(())
	}
}
