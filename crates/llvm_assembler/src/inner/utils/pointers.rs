use frick_assembler::TAPE_SIZE;
use inkwell::{
	builder::Builder, module::Module, targets::TargetData, types::IntType, values::PointerValue,
};

use super::AssemblerFunctions;
use crate::LlvmAssemblyError;

#[derive(Debug, Clone, Copy)]
pub struct AssemblerPointers<'ctx> {
	pub tape: PointerValue<'ctx>,
	pub pointer: PointerValue<'ctx>,
	pub puts_alloca: PointerValue<'ctx>,
	pub single_puts_alloca: PointerValue<'ctx>,
}

impl<'ctx> AssemblerPointers<'ctx> {
	pub fn new(
		module: &Module<'ctx>,
		functions: AssemblerFunctions<'ctx>,
		builder: &Builder<'ctx>,
		target_data: &TargetData,
	) -> Result<(Self, IntType<'ctx>), LlvmAssemblyError> {
		let context = module.get_context();
		let i8_type = context.i8_type();
		let i64_type = context.i64_type();
		let ptr_int_type = context.ptr_sized_int_type(target_data, None);

		let tape = {
			let i8_array_type = i8_type.array_type(TAPE_SIZE as u32);
			let i8_array_size = i64_type.const_int(TAPE_SIZE as u64, false);

			let tape_alloca = builder.build_alloca(i8_array_type, "tape")?;

			builder.build_call(
				functions.lifetime.start,
				&[i8_array_size.into(), tape_alloca.into()],
				"",
			)?;

			builder.build_memset(tape_alloca, 1, i8_type.const_zero(), i8_array_size)?;

			tape_alloca
		};

		let pointer = {
			let pointer_alloca = builder.build_alloca(ptr_int_type, "pointer")?;
			let pointer_size = i64_type.const_int(8, false);

			builder.build_call(
				functions.lifetime.start,
				&[pointer_size.into(), pointer_alloca.into()],
				"",
			)?;

			builder.build_store(pointer_alloca, ptr_int_type.const_zero())?;

			pointer_alloca
		};

		let puts_alloca = {
			let i8_array_type = i8_type.array_type(8);

			builder.build_alloca(i8_array_type, "puts_output")?
		};

		let single_puts_alloca = {
			let i8_array_type = i8_type.array_type(2);

			let alloca = builder.build_alloca(i8_array_type, "single_puts_output")?;

			let second_index = i64_type.const_int(1, false);

			let gep = unsafe { builder.build_in_bounds_gep(i8_type, alloca, &[second_index], "")? };

			builder.build_store(gep, i8_type.const_zero())?;

			alloca
		};

		Ok((
			Self {
				tape,
				pointer,
				puts_alloca,
				single_puts_alloca,
			},
			ptr_int_type,
		))
	}
}
