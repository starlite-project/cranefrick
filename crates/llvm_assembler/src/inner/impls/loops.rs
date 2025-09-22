use frick_assembler::AssemblyError;
use frick_ir::BrainIr;
use inkwell::IntPredicate;

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn if_not_zero(&self, ops: &[BrainIr]) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		let preheader_block = self
			.context()
			.append_basic_block(self.functions.main, "if_not_zero.preheader");
		let header_block = self
			.context()
			.append_basic_block(self.functions.main, "if_not_zero.header");
		let body_block = self
			.context()
			.append_basic_block(self.functions.main, "if_not_zero.body");
		let exit_block = self
			.context()
			.append_basic_block(self.functions.main, "if_not_zero.exit");

		self.builder
			.build_unconditional_branch(preheader_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(preheader_block);

		self.builder
			.build_unconditional_branch(header_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(header_block);

		let value = self.load(0, "if_not_zero")?;

		let zero = {
			let i8_type = self.context().i8_type();

			i8_type.const_zero()
		};

		let cmp = self
			.builder
			.build_int_compare(IntPredicate::NE, value, zero, "if_not_zero_cmp")
			.map_err(AssemblyError::backend)?;

		self.builder
			.build_conditional_branch(cmp, body_block, exit_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(body_block);

		self.ops(ops)?;

		self.builder
			.build_unconditional_branch(exit_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(exit_block);

		Ok(())
	}

	pub fn dynamic_loop(&self, ops: &[BrainIr]) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		let preheader_block = self
			.context()
			.append_basic_block(self.functions.main, "dynamic_loop.preheader");
		let header_block = self
			.context()
			.append_basic_block(self.functions.main, "dynamic_loop.header");
		let body_block = self
			.context()
			.append_basic_block(self.functions.main, "dynamic_loop.body");
		let exit_block = self
			.context()
			.append_basic_block(self.functions.main, "dynamic_loop.exit");

		self.builder
			.build_unconditional_branch(preheader_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(preheader_block);

		self.builder
			.build_unconditional_branch(header_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(header_block);

		let value = self.load(0, "dynamic_loop")?;

		let zero = {
			let i8_type = self.context().i8_type();

			i8_type.const_zero()
		};

		let cmp = self
			.builder
			.build_int_compare(IntPredicate::NE, value, zero, "dynamic_loop_cmp")
			.map_err(AssemblyError::backend)?;

		self.builder
			.build_conditional_branch(cmp, body_block, exit_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(body_block);

		self.ops(ops)?;

		self.builder
			.build_unconditional_branch(header_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(exit_block);

		Ok(())
	}

	pub fn find_zero(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let preheader_block = self
			.context()
			.append_basic_block(self.functions.main, "find_zero.preheader");
		let header_block = self
			.context()
			.append_basic_block(self.functions.main, "find_zero.header");
		let body_block = self
			.context()
			.append_basic_block(self.functions.main, "find_zero.body");
		let exit_block = self
			.context()
			.append_basic_block(self.functions.main, "find_zero.exit");

		self.builder.build_unconditional_branch(preheader_block)?;

		self.builder.position_at_end(preheader_block);

		self.builder.build_unconditional_branch(header_block)?;

		self.builder.position_at_end(header_block);

		let value = self.load(0, "find_zero")?;

		let zero = {
			let i8_type = self.context().i8_type();

			i8_type.const_zero()
		};

		let cmp = self
			.builder
			.build_int_compare(IntPredicate::NE, value, zero, "find_zero_cmp")?;

		self.builder
			.build_conditional_branch(cmp, body_block, exit_block)?;

		self.builder.position_at_end(body_block);

		self.move_pointer(offset)?;

		self.builder.build_unconditional_branch(header_block)?;

		self.builder.position_at_end(exit_block);

		Ok(())
	}
}
