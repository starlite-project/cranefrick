use frick_spec::TAPE_SIZE;
use frick_utils::{Convert as _, InsertOrPush as _};
use inkwell::{
	IntPredicate,
	attributes::AttributeLoc,
	values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, PointerValue},
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

	pub(super) fn store_immediate_into_cell(&self, imm: u8) -> Result<(), AssemblyError> {
		let cell_type = self.context().i8_type();

		let gep = self.index_tape()?;

		let cell_value = cell_type.const_int(imm.convert::<u64>(), false);

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

		let cell_type = context.i8_type();
		let call_site_value = self
			.builder
			.build_direct_call(self.functions.getchar, &[], "\0")?;

		let call_value = call_site_value
			.try_as_basic_value()
			.unwrap_basic()
			.into_int_value();

		let truncated_value = self
			.builder
			.build_int_truncate_or_bit_cast(call_value, cell_type, "\0")?;

		self.set_value_at(reg, truncated_value)?;

		Ok(())
	}

	pub(super) fn output_from_register(&self, reg: usize) -> Result<(), AssemblyError> {
		let context = self.context();

		let register_value = self.value_at(reg)?;

		let call_value = self.builder.build_direct_call(
			self.functions.putchar,
			&[register_value.convert::<BasicMetadataValueEnum<'ctx>>()],
			"\0",
		)?;

		let zeroext_attr = context.create_named_enum_attribute("zeroext", 0);

		call_value.add_attribute(AttributeLoc::Param(0), zeroext_attr);
		call_value.set_tail_call(true);

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

		let tape_size = pointer_ty.const_int(TAPE_SIZE as u64, false);

		self.pointer_register.borrow_mut().replace({
			let added_pointer_value =
				self.builder
					.build_int_add(pointer_value, offset_value, "\0")?;

			self.builder.build_and(
				added_pointer_value,
				tape_size.const_sub(pointer_ty.const_int(1, false)),
				"\0",
			)?
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

	pub(super) fn compare_reg_to_immediate(
		&self,
		input_reg: usize,
		output_reg: usize,
		imm: u8,
	) -> Result<(), AssemblyError> {
		let cell_type = self.context().i8_type();

		let compare_value = cell_type.const_int(imm.convert::<u64>(), false);

		let input_value = self.value_at(input_reg)?.into_int_value();

		let output =
			self.builder
				.build_int_compare(IntPredicate::EQ, input_value, compare_value, "\0")?;

		self.set_value_at(output_reg, output)?;

		Ok(())
	}

	pub(super) fn jump_if(&self, input_reg: usize) -> Result<(), AssemblyError> {
		let loop_info = self.last_loop_info()?;

		let compare_value = self.value_at(input_reg)?.into_int_value();

		self.builder
			.build_conditional_branch(compare_value, loop_info.exit, loop_info.body)?;
		self.builder.position_at_end(loop_info.body);

		Ok(())
	}

	pub(super) fn jump_to_header(&self) -> Result<(), AssemblyError> {
		let loop_info = self.last_loop_info()?;

		self.builder.build_unconditional_branch(loop_info.header)?;

		Ok(())
	}

	fn index_tape(&self) -> Result<PointerValue<'ctx>, AssemblyError> {
		let cell_type = self.context().i8_type();
		let ptr_int_type = self.pointers.pointer_ty;
		let tape_type = cell_type.array_type(TAPE_SIZE as u32);

		let pointer_value = self
			.pointer_register
			.borrow()
			.ok_or(AssemblyError::PointerNotLoaded)?;

		Ok(unsafe {
			self.builder.build_in_bounds_gep(
				tape_type,
				self.pointers.tape,
				&[ptr_int_type.const_zero(), pointer_value],
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

	fn last_loop_info(&self) -> Result<LoopBlocks<'ctx>, AssemblyError> {
		self.loop_blocks
			.borrow()
			.last()
			.copied()
			.ok_or(AssemblyError::NoLoopInfo)
	}
}
