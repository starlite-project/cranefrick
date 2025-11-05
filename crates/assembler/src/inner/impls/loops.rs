use frick_ir::BrainIr;
use inkwell::{IntPredicate, debug_info::AsDIScope as _};

use crate::{AssemblyError, ContextGetter as _, inner::InnerAssembler};

impl InnerAssembler<'_> {
	pub fn if_not_zero(&self, ops: &[BrainIr], op_count: &mut usize) -> Result<(), AssemblyError> {
		let context = self.context();

		let header_block = context.append_basic_block(self.functions.main, "if_not_zero.header\0");
		let body_block = context.append_basic_block(self.functions.main, "if_not_zero.body\0");
		let exit_block = context.append_basic_block(self.functions.main, "if_not_zero.exit\0");

		self.builder.build_unconditional_branch(header_block)?;

		self.builder.position_at_end(header_block);

		let value = self.load_cell(0)?;

		let zero = {
			let i8_type = context.i8_type();

			i8_type.const_zero()
		};

		let cmp =
			self.builder
				.build_int_compare(IntPredicate::NE, value, zero, "if_not_zero_cmp\0")?;

		self.builder
			.build_conditional_branch(cmp, body_block, exit_block)?;

		self.builder.position_at_end(body_block);

		*op_count += 1;

		self.ops(ops, op_count)?;

		*op_count -= 1;

		let debug_loc = self.debug_builder.create_debug_location(
			context,
			1,
			*op_count as u32 + 2,
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

	pub fn dynamic_loop(&self, ops: &[BrainIr], op_count: &mut usize) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();

		let preheader_block =
			context.append_basic_block(self.functions.main, "dynamic_loop.preheader\0");
		let header_block = context.append_basic_block(self.functions.main, "dynamic_loop.header\0");
		let body_block = context.append_basic_block(self.functions.main, "dynamic_loop.body\0");
		let latch_block = context.append_basic_block(self.functions.main, "dynamic_loop.latch\0");
		let exit_block = context.append_basic_block(self.functions.main, "dynamic_loop.exit\0");

		self.builder.build_unconditional_branch(preheader_block)?;
		self.builder.position_at_end(preheader_block);

		let current_value = self.load_cell(0)?;

		self.builder.build_unconditional_branch(header_block)?;
		self.builder.position_at_end(header_block);

		let loaded_value_phi = self.builder.build_phi(i8_type, "loaded_value\0")?;

		let zero = {
			let i8_type = context.i8_type();

			i8_type.const_zero()
		};

		let cmp = self.builder.build_int_compare(
			IntPredicate::NE,
			loaded_value_phi.as_basic_value().into_int_value(),
			zero,
			"dynamic_loop_cmp\0",
		)?;

		self.builder
			.build_conditional_branch(cmp, body_block, exit_block)?;
		self.builder.position_at_end(body_block);

		*op_count += 1;

		self.ops(ops, op_count)?;

		*op_count -= 1;

		let debug_loc = self.debug_builder.create_debug_location(
			context,
			1,
			*op_count as u32 + 2,
			self.functions
				.main
				.get_subprogram()
				.unwrap()
				.as_debug_info_scope(),
			None,
		);

		self.builder.set_current_debug_location(debug_loc);

		self.builder.build_unconditional_branch(latch_block)?;
		self.builder.position_at_end(latch_block);

		let new_cell_value = self.load_cell(0)?;

		loaded_value_phi.add_incoming(&[
			(&current_value, preheader_block),
			(&new_cell_value, latch_block),
		]);

		self.builder.build_unconditional_branch(header_block)?;
		self.builder.position_at_end(exit_block);

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub fn find_zero(&self, offset: i32) -> Result<(), AssemblyError> {
		let context = self.context();

		let ptr_int_type = self.pointers.pointer_ty;
		let i8_type = context.i8_type();

		let _invariant_start = self.start_tape_invariant()?;

		let preheader_block =
			context.append_basic_block(self.functions.main, "find_zero.preheader\0");
		let header_block = context.append_basic_block(self.functions.main, "find_zero.header\0");
		let body_block = context.append_basic_block(self.functions.main, "find_zero.body\0");
		let exit_block = context.append_basic_block(self.functions.main, "find_zero.exit\0");

		self.builder.build_unconditional_branch(preheader_block)?;
		self.builder.position_at_end(preheader_block);

		let current_pointer_value = self.load_from(ptr_int_type, self.pointers.pointer)?;

		self.builder.build_unconditional_branch(header_block)?;
		self.builder.position_at_end(header_block);

		let header_phi_value = self.builder.build_phi(ptr_int_type, "find_zero_phi\0")?;

		let gep = self.tape_gep(i8_type, header_phi_value.as_basic_value().into_int_value())?;

		let value = self.load_from(i8_type, gep)?;

		let zero = i8_type.const_zero();

		let cmp =
			self.builder
				.build_int_compare(IntPredicate::NE, value, zero, "find_zero_cmp\0")?;

		self.builder
			.build_conditional_branch(cmp, body_block, exit_block)?;

		self.builder.position_at_end(body_block);

		let offset_value = ptr_int_type.const_int(offset as u64, false);

		let new_pointer_value = self.builder.build_int_add(
			header_phi_value.as_basic_value().into_int_value(),
			offset_value,
			"find_zero_add\0",
		)?;

		let wrapped_pointer_value = self.wrap_pointer(new_pointer_value, offset > 0)?;

		self.builder.build_unconditional_branch(header_block)?;

		header_phi_value.add_incoming(&[
			(&current_pointer_value, preheader_block),
			(&wrapped_pointer_value, body_block),
		]);

		self.builder.position_at_end(exit_block);

		let store_instr =
			self.store_into(header_phi_value.as_basic_value(), self.pointers.pointer)?;

		let current_debug_loc = self.builder.get_current_debug_location().unwrap();

		self.pointer_setting_instructions.borrow_mut().push((
			store_instr,
			header_phi_value.as_basic_value().into_int_value(),
			current_debug_loc,
		));

		Ok(())
	}
}
