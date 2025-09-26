use frick_assembler::{AssemblyError, TAPE_SIZE};
use frick_ir::BrainIr;
use inkwell::IntPredicate;

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn if_not_zero(&self, ops: &[BrainIr], ops_count: usize) -> Result<(), AssemblyError<LlvmAssemblyError>> {
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

		self.ops(ops, ops_count + 1)?;

		self.builder
			.build_unconditional_branch(exit_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(exit_block);

		Ok(())
	}

	pub fn dynamic_loop(&self, ops: &[BrainIr], ops_count: usize) -> Result<(), AssemblyError<LlvmAssemblyError>> {
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

		self.ops(ops, ops_count + 1)?;

		self.builder
			.build_unconditional_branch(header_block)
			.map_err(AssemblyError::backend)?;

		self.builder.position_at_end(exit_block);

		Ok(())
	}

	pub fn find_zero(&self, offset: i32) -> Result<(), LlvmAssemblyError> {
		let current_block = self.builder.get_insert_block().unwrap();

		let ptr_int_type = self.ptr_int_type;
		let i8_type = self.context().i8_type();

		let current_pointer_value = self
			.builder
			.build_load(
				ptr_int_type,
				self.pointers.pointer,
				"find_zero_load_pointer",
			)?
			.into_int_value();

		let header_block = self
			.context()
			.append_basic_block(self.functions.main, "find_zero.header");
		let body_block = self
			.context()
			.append_basic_block(self.functions.main, "find_zero.body");
		let exit_block = self
			.context()
			.append_basic_block(self.functions.main, "find_zero.exit");

		self.builder.build_unconditional_branch(header_block)?;

		self.builder.position_at_end(header_block);

		let header_phi_value = self.builder.build_phi(ptr_int_type, "find_zero_phi")?;

		header_phi_value.add_incoming(&[(&current_pointer_value, current_block)]);

		let gep = self.gep(
			i8_type,
			header_phi_value.as_basic_value().into_int_value(),
			"find_zero",
		)?;

		let value = self
			.builder
			.build_load(i8_type, gep, "find_zero_cell_load")?
			.into_int_value();

		let zero = i8_type.const_zero();

		let cmp = self
			.builder
			.build_int_compare(IntPredicate::NE, value, zero, "find_zero_cmp")?;

		self.builder
			.build_conditional_branch(cmp, body_block, exit_block)?;

		self.builder.position_at_end(body_block);

		let offset_value = ptr_int_type.const_int(offset as u64, false);

		let new_pointer_value = self.builder.build_int_add(
			header_phi_value.as_basic_value().into_int_value(),
			offset_value,
			"find_zero_add",
		)?;

		let wrapped_pointer_value = {
			let tape_len = ptr_int_type.const_int(TAPE_SIZE as u64 - 1, false);

			self.builder
				.build_and(new_pointer_value, tape_len, "find_zero_and")?
		};

		self.builder.build_unconditional_branch(header_block)?;

		header_phi_value.add_incoming(&[(&wrapped_pointer_value, body_block)]);

		self.builder.position_at_end(exit_block);

		self.builder.build_store(
			self.pointers.pointer,
			header_phi_value.as_basic_value().into_int_value(),
		)?;

		Ok(())
	}
}
