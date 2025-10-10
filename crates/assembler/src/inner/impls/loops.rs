use frick_ir::BrainIr;
use frick_spec::TAPE_SIZE;
use inkwell::{IntPredicate, debug_info::AsDIScope as _};

use crate::{AssemblyError, ContextGetter as _, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn if_not_zero(&self, ops: &[BrainIr], op_count: usize) -> Result<(), AssemblyError> {
		let context = self.context();

		let header_block = context.append_basic_block(self.functions.main, "if_not_zero.header");
		let body_block = context.append_basic_block(self.functions.main, "if_not_zero.body");
		let exit_block = context.append_basic_block(self.functions.main, "if_not_zero.exit");

		self.builder.build_unconditional_branch(header_block)?;

		self.builder.position_at_end(header_block);

		let value = self.load(0, "if_not_zero")?;

		let zero = {
			let i8_type = context.i8_type();

			i8_type.const_zero()
		};

		let cmp =
			self.builder
				.build_int_compare(IntPredicate::NE, value, zero, "if_not_zero_cmp")?;

		self.builder
			.build_conditional_branch(cmp, body_block, exit_block)?;

		self.builder.position_at_end(body_block);

		self.ops(ops, op_count + 1)?;

		let debug_loc = self.debug_builder.create_debug_location(
			context,
			0,
			op_count as u32 + 2,
			self.functions
				.main
				.get_subprogram()
				.unwrap()
				.as_debug_info_scope(),
			None,
		);

		self.builder.set_current_debug_location(debug_loc);

		self.builder.build_unconditional_branch(exit_block)?;

		self.builder.position_at_end(exit_block);

		Ok(())
	}

	pub fn dynamic_loop(&self, ops: &[BrainIr], op_count: usize) -> Result<(), AssemblyError> {
		let context = self.context();

		let header_block = context.append_basic_block(self.functions.main, "dynamic_loop.header");
		let body_block = context.append_basic_block(self.functions.main, "dynamic_loop.body");
		let exit_block = context.append_basic_block(self.functions.main, "dynamic_loop.exit");

		self.builder.build_unconditional_branch(header_block)?;

		self.builder.position_at_end(header_block);

		let value = self.load(0, "dynamic_loop")?;

		let zero = {
			let i8_type = context.i8_type();

			i8_type.const_zero()
		};

		let cmp =
			self.builder
				.build_int_compare(IntPredicate::NE, value, zero, "dynamic_loop_cmp")?;

		self.builder
			.build_conditional_branch(cmp, body_block, exit_block)?;

		self.builder.position_at_end(body_block);

		self.ops(ops, op_count + 1)?;

		let debug_loc = self.debug_builder.create_debug_location(
			context,
			0,
			op_count as u32 + 2,
			self.functions
				.main
				.get_subprogram()
				.unwrap()
				.as_debug_info_scope(),
			None,
		);

		self.builder.set_current_debug_location(debug_loc);

		self.builder.build_unconditional_branch(header_block)?;

		self.builder.position_at_end(exit_block);

		Ok(())
	}

	pub fn find_zero(&self, offset: i32) -> Result<(), AssemblyError> {
		let context = self.context();

		let current_block = self.builder.get_insert_block().unwrap();

		let ptr_int_type = self.ptr_int_type;
		let i8_type = context.i8_type();

		let current_pointer_value = self
			.builder
			.build_load(
				ptr_int_type,
				self.pointers.pointer,
				"find_zero_load_pointer",
			)?
			.into_int_value();

		let header_block = context.append_basic_block(self.functions.main, "find_zero.header");
		let body_block = context.append_basic_block(self.functions.main, "find_zero.body");
		let exit_block = context.append_basic_block(self.functions.main, "find_zero.exit");

		self.builder.build_unconditional_branch(header_block)?;

		self.builder.position_at_end(header_block);

		let header_phi_value = self.builder.build_phi(ptr_int_type, "find_zero_phi")?;

		let gep = self.tape_gep(
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

		header_phi_value.add_incoming(&[
			(&current_pointer_value, current_block),
			(&wrapped_pointer_value, body_block),
		]);

		self.builder.position_at_end(exit_block);

		self.builder.build_store(
			self.pointers.pointer,
			header_phi_value.as_basic_value().into_int_value(),
		)?;

		Ok(())
	}
}
