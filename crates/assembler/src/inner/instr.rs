use frick_instructions::Imm;
use frick_spec::POINTER_SIZE;
use frick_types::{Any, BinaryOperation, Bool, Int, Pointer, Register};
use frick_utils::Convert as _;
use inkwell::{
	IntPredicate,
	attributes::AttributeLoc,
	llvm_sys::{LLVMGEPFlagInBounds, LLVMGEPFlagNUW},
	values::{BasicMetadataValueEnum, BasicValue, LLVMTailCallKind},
};

use super::{AssemblyError, InnerAssembler, LoopBlocks, utils::Castable};
use crate::{BuilderExt as _, ContextExt, ContextGetter as _};

impl<'ctx> InnerAssembler<'ctx> {
	pub(super) fn load_cell_into_register(
		&self,
		pointer_reg: Register<Pointer>,
		output_reg: Register<Int>,
	) -> Result<(), AssemblyError> {
		let cell_type = self.context().i8_type();

		let ptr_value = self.value_at(pointer_reg)?;

		let cell_value = self.builder.build_load(cell_type, ptr_value, "\0")?;

		self.set_value_at(output_reg, cell_value)
	}

	pub(super) fn store_register_into_cell(
		&self,
		value_reg: Register<Int>,
		pointer_reg: Register<Pointer>,
	) -> Result<(), AssemblyError> {
		let ptr_value = self.value_at(pointer_reg)?;

		let cell_value = self.value_at(value_reg)?;

		self.builder.build_store(ptr_value, cell_value)?;

		Ok(())
	}

	pub(super) fn store_immediate_into_register(
		&self,
		output_reg: Register<Int>,
		imm: Imm,
	) -> Result<(), AssemblyError> {
		let int_type = self.context().custom_width_int_type(imm.size());

		let value = int_type.const_int(imm.value(), false);

		self.set_value_at(output_reg, value)
	}

	pub(super) fn load_tape_pointer_into_register(
		&self,
		output_reg: Register<Int>,
	) -> Result<(), AssemblyError> {
		let pointer_type = self.context().custom_width_int_type(POINTER_SIZE as u32);

		let value = self
			.builder
			.build_load(pointer_type, self.pointers.pointer, "\0")?;

		self.set_value_at(output_reg, value)
	}

	pub(super) fn store_register_into_tape_pointer(
		&self,
		input_reg: Register<Int>,
	) -> Result<(), AssemblyError> {
		let ptr_value = self.value_at(input_reg)?;

		self.builder.build_store(self.pointers.pointer, ptr_value)?;

		Ok(())
	}

	pub(super) fn calculate_tape_offset(
		&self,
		input_reg: Register<Int>,
		output_reg: Register<Pointer>,
	) -> Result<(), AssemblyError> {
		let cell_type = self.context().i8_type();
		let pointer_value = self.value_at(input_reg)?;

		let offset_pointer = unsafe {
			self.builder.build_gep_with_no_wrap_flags(
				cell_type,
				self.pointers.tape,
				&[pointer_value],
				"\0",
				LLVMGEPFlagInBounds | LLVMGEPFlagNUW,
			)?
		};

		self.set_value_at(output_reg, offset_pointer)
	}

	pub(super) fn perform_binary_register_operation(
		&self,
		lhs: Register<Int>,
		rhs: Register<Int>,
		output_reg: Register<Int>,
		op: BinaryOperation,
	) -> Result<(), AssemblyError> {
		let lhs_value = self.value_at::<Int>(lhs)?;
		let rhs_value = self.value_at::<Int>(rhs)?;

		let new_value = match op {
			BinaryOperation::Add => self.builder.build_int_add(lhs_value, rhs_value, "\0")?,
			BinaryOperation::Sub => self.builder.build_int_sub(lhs_value, rhs_value, "\0")?,
			BinaryOperation::Mul => self.builder.build_int_mul(lhs_value, rhs_value, "\0")?,
			BinaryOperation::BitwiseAnd => self.builder.build_and(lhs_value, rhs_value, "\0")?,
			BinaryOperation::BitwiseShl => {
				self.builder.build_left_shift(lhs_value, rhs_value, "\0")?
			}
			op => unimplemented!("binary operation {op:?}"),
		};

		self.set_value_at(output_reg, new_value)
	}

	pub(super) fn duplicate_register(
		&self,
		input_reg: Register<Any>,
		output_reg: Register<Any>,
	) -> Result<(), AssemblyError> {
		let input = self.value_at(input_reg)?;

		self.set_value_at(output_reg, input)
	}

	pub(super) fn input_into_register(&self, reg: Register<Int>) -> Result<(), AssemblyError> {
		let context = self.context();

		let cell_type = context.i8_type();
		let call_site_value = self
			.builder
			.build_direct_call(self.functions.getchar, &[], "\0")?;

		call_site_value.set_tail_call_kind(LLVMTailCallKind::LLVMTailCallKindNoTail);

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

	pub(super) fn output_from_register(&self, reg: Register<Int>) -> Result<(), AssemblyError> {
		let context = self.context();

		let register_value = self.value_at(reg)?;

		let call_site_value = self.builder.build_direct_call(
			self.functions.putchar,
			&[register_value.convert::<BasicMetadataValueEnum<'ctx>>()],
			"\0",
		)?;

		let zeroext_attr = context.create_named_enum_attribute("zeroext", 0);

		call_site_value.add_attribute(AttributeLoc::Param(0), zeroext_attr);
		call_site_value.set_tail_call_kind(LLVMTailCallKind::LLVMTailCallKindTail);

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

	pub(super) fn compare_register_to_register(
		&self,
		lhs: Register<Int>,
		rhs: Register<Int>,
		output_reg: Register<Bool>,
	) -> Result<(), AssemblyError> {
		let lhs_value = self.value_at(lhs)?;
		let rhs_value = self.value_at(rhs)?;

		let output =
			self.builder
				.build_int_compare(IntPredicate::EQ, lhs_value, rhs_value, "\0")?;

		self.set_value_at(output_reg, output)
	}

	pub(super) fn jump_if(&self, input_reg: Register<Bool>) -> Result<(), AssemblyError> {
		let loop_info = self.last_loop_info()?;

		let compare_value = self.value_at(input_reg)?;

		let br_instr =
			self.builder
				.build_conditional_branch(compare_value, loop_info.exit, loop_info.body)?;
		self.builder.position_at_end(loop_info.body);

		self.add_loop_metadata_to_br(br_instr)
	}

	pub(super) fn jump_to_header(&self) -> Result<(), AssemblyError> {
		let loop_info = self.last_loop_info()?;

		self.builder.build_unconditional_branch(loop_info.header)?;

		Ok(())
	}

	fn value_at<T: Castable<'ctx>>(&self, reg: Register<T>) -> Result<T::Value, AssemblyError> {
		let basic_value = self
			.registers
			.borrow()
			.get(&reg.index())
			.copied()
			.ok_or_else(|| AssemblyError::NoValueInRegister(reg.index()))?;

		T::assert_type_matches(basic_value);

		Ok(T::cast(basic_value))
	}

	fn set_value_at<T: Castable<'ctx>, V>(
		&self,
		reg: Register<T>,
		value: V,
	) -> Result<(), AssemblyError>
	where
		V: BasicValue<'ctx> + Copy,
	{
		T::assert_type_matches(value);

		self.registers
			.borrow_mut()
			.insert(reg.index(), value.as_basic_value_enum());

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
