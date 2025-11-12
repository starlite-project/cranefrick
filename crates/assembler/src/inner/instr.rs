use frick_spec::TAPE_SIZE;
use frick_utils::InsertOrPush as _;
use inkwell::{
	IntPredicate,
	attributes::AttributeLoc,
	values::{BasicValue, BasicValueEnum, PointerValue},
};

use super::{AssemblyError, InnerAssembler, LoopBlocks};
use crate::{ContextExt, ContextGetter as _};

impl<'ctx> InnerAssembler<'ctx> {
	pub(super) fn load_cell_into_register(&self, reg: usize) -> Result<(), AssemblyError> {
		let cell_type = self.context().i8_type();

		let gep = self.index_tape()?;

		let cell_value = self.builder.build_load(cell_type, gep, "\0")?;

		self.set_value_at(reg, cell_value)
	}

	pub(super) fn store_register_into_cell(&self, reg: usize) -> Result<(), AssemblyError> {
		let gep = self.index_tape()?;

		let cell_value = self.value_at(reg)?;

		self.builder.build_store(gep, cell_value)?;

		Ok(())
	}

	pub(super) fn change_register_by_immediate(
		&self,
		reg: usize,
		value: i8,
	) -> Result<(), AssemblyError> {
		let cell_type = self.context().i8_type();

		let register_value = self.value_at(reg)?.into_int_value();

		let new_value = self.builder.build_int_add(
			register_value,
			cell_type.const_int(value as u64, false),
			"\0",
		)?;

		self.set_value_at(reg, new_value)
	}

	pub(super) fn input_into_register(&self, reg: usize) -> Result<(), AssemblyError> {
		let context = self.context();

		let continue_block =
			context.append_basic_block(self.functions.main, "input_into_register.continue\0");

		let cell_type = context.i8_type();

		let call_site_value = self.builder.build_direct_invoke(
			self.functions.getchar,
			&[],
			continue_block,
			self.catch_block,
			"\0",
		)?;

		let call_value = call_site_value
			.try_as_basic_value()
			.unwrap_basic()
			.into_int_value();

		let truncated_value = self
			.builder
			.build_int_truncate_or_bit_cast(call_value, cell_type, "\0")?;

		self.set_value_at(reg, truncated_value)?;

		self.builder.position_at_end(continue_block);

		Ok(())
	}

	pub(super) fn output_from_register(&self, reg: usize) -> Result<(), AssemblyError> {
		let context = self.context();

		let continue_block =
			context.append_basic_block(self.functions.main, "output_from_register.continue\0");

		let register_value = self.value_at(reg)?;

		let call = self.builder.build_direct_invoke(
			self.functions.putchar,
			&[register_value],
			continue_block,
			self.catch_block,
			"\0",
		)?;

		let zeroext_attr = context.create_named_enum_attribute("zeroext", 0);

		call.add_attribute(AttributeLoc::Param(0), zeroext_attr);
		call.set_tail_call(true);

		self.builder.position_at_end(continue_block);

		Ok(())
	}

	pub(super) fn load_pointer(&self) -> Result<(), AssemblyError> {
		self.pointer_register.borrow_mut().replace(
			self.builder
				.build_load(self.pointers.pointer_ty, self.pointers.pointer, "\0")?
				.into_int_value(),
		);

		Ok(())
	}

	pub(super) fn offset_pointer(&self, offset: i32) -> Result<(), AssemblyError> {
		let pointer_ty = self.pointers.pointer_ty;

		let pointer_value = self
			.pointer_register
			.borrow()
			.ok_or(AssemblyError::PointerNotLoaded)?;

		let offset_value = pointer_ty.const_int(offset as u64, false);

		self.pointer_register.borrow_mut().replace({
			let added_pointer_value =
				self.builder
					.build_int_add(pointer_value, offset_value, "\0")?;
			let tape_size = pointer_ty.const_int(TAPE_SIZE as u64, false);

			if offset > 0 {
				self.builder
					.build_int_unsigned_rem(added_pointer_value, tape_size, "\0")?
			} else {
				let tmp =
					self.builder
						.build_int_signed_rem(added_pointer_value, tape_size, "\0")?;

				let added_offset = self.builder.build_int_add(tmp, tape_size, "\0")?;

				let cmp = self.builder.build_int_compare(
					IntPredicate::SLT,
					tmp,
					pointer_ty.const_zero(),
					"\0",
				)?;

				self.builder
					.build_select(cmp, added_offset, tmp, "\0")?
					.into_int_value()
			}
		});

		Ok(())
	}

	pub(super) fn store_pointer(&self) -> Result<(), AssemblyError> {
		let pointer_value = self
			.pointer_register
			.borrow_mut()
			.take()
			.ok_or(AssemblyError::PointerNotLoaded)?;

		self.builder
			.build_store(self.pointers.pointer, pointer_value)?;

		Ok(())
	}

	pub(super) fn start_loop(&self) -> Result<(), AssemblyError> {
		let loop_blocks = {
			let context = self.context();

			let header = context.append_basic_block(self.functions.main, "loop.header\0");
			let body = context.append_basic_block(self.functions.main, "loop.body\0");
			let exit = context.append_basic_block(self.functions.main, "loop.exit\0");

			LoopBlocks { header, body, exit }
		};

		self.loop_blocks.borrow_mut().push(loop_blocks);

		self.builder
			.build_unconditional_branch(loop_blocks.header)?;
		self.builder.position_at_end(loop_blocks.header);

		Ok(())
	}

	pub(super) fn end_loop(&self) -> Result<(), AssemblyError> {
		let loop_info = self
			.loop_blocks
			.borrow_mut()
			.pop()
			.ok_or(AssemblyError::NoLoopInfo)?;

		self.builder.position_at_end(loop_info.exit);

		Ok(())
	}

	pub(super) fn jump_if_zero(&self, reg: usize) -> Result<(), AssemblyError> {
		let cell_type = self.context().i8_type();

		let reg_value = self.value_at(reg)?.into_int_value();

		let cell_zero = cell_type.const_zero();

		let comparison =
			self.builder
				.build_int_compare(IntPredicate::EQ, reg_value, cell_zero, "\0")?;

		let loop_info = self
			.loop_blocks
			.borrow()
			.last()
			.copied()
			.ok_or(AssemblyError::NoLoopInfo)?;

		self.builder
			.build_conditional_branch(comparison, loop_info.exit, loop_info.body)?;
		self.builder.position_at_end(loop_info.body);

		Ok(())
	}

	pub(super) fn jump_if_not_zero(&self, reg: usize) -> Result<(), AssemblyError> {
		let cell_type = self.context().i8_type();

		let reg_value = self.value_at(reg)?.into_int_value();

		let cell_zero = cell_type.const_zero();

		let comparison =
			self.builder
				.build_int_compare(IntPredicate::NE, reg_value, cell_zero, "\0")?;

		let loop_info = self
			.loop_blocks
			.borrow()
			.last()
			.copied()
			.ok_or(AssemblyError::NoLoopInfo)?;

		self.builder
			.build_conditional_branch(comparison, loop_info.body, loop_info.exit)?;

		self.builder.position_at_end(loop_info.body);

		Ok(())
	}

	fn index_tape(&self) -> Result<PointerValue<'ctx>, AssemblyError> {
		let cell_type = self.context().i8_type();

		let pointer_value = self
			.pointer_register
			.borrow()
			.ok_or(AssemblyError::PointerNotLoaded)?;

		Ok(unsafe {
			self.builder.build_in_bounds_gep(
				cell_type,
				self.pointers.tape,
				&[pointer_value],
				"\0",
			)?
		})
	}

	fn value_at(&self, reg: usize) -> Result<BasicValueEnum<'ctx>, AssemblyError> {
		self.registers
			.borrow()
			.get(reg)
			.copied()
			.ok_or_else(|| AssemblyError::NoValueInRegister(reg))
	}

	fn set_value_at(&self, reg: usize, value: impl BasicValue<'ctx>) -> Result<(), AssemblyError> {
		self.registers
			.borrow_mut()
			.insert_or_push(reg, value.as_basic_value_enum());

		Ok(())
	}
}
