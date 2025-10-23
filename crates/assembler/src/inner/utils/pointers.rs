use frick_spec::TAPE_SIZE;
use inkwell::{
	builder::Builder, context::AsContextRef, llvm_sys::prelude::LLVMContextRef, module::Module,
	targets::TargetData, types::IntType, values::PointerValue,
};

use super::AssemblerFunctions;
use crate::AssemblyError;

#[derive(Debug, Clone, Copy)]
pub struct AssemblerPointers<'ctx> {
	pub tape: PointerValue<'ctx>,
	pub pointer: PointerValue<'ctx>,
	pub output: PointerValue<'ctx>,
}

impl<'ctx> AssemblerPointers<'ctx> {
	pub fn new(
		module: &Module<'ctx>,
		functions: &AssemblerFunctions<'ctx>,
		builder: &Builder<'ctx>,
		target_data: &TargetData,
	) -> Result<(Self, IntType<'ctx>), AssemblyError> {
		let context = module.get_context();
		let i8_type = context.i8_type();
		let i64_type = context.i64_type();
		let ptr_int_type = context.ptr_sized_int_type(target_data, None);

		let i8_zero = i8_type.const_zero();

		let tape = {
			let i8_array_type = i8_type.array_type(TAPE_SIZE as u32);
			let i8_array_size = i64_type.const_int(TAPE_SIZE as u64, false);

			let tape_alloca = builder.build_alloca(i8_array_type, "tape")?;

			if let Some(tape_instr) = tape_alloca.as_instruction() {
				tape_instr.set_alignment(16)?;
			}

			builder.build_call(
				functions.lifetime.start,
				&[i8_array_size.into(), tape_alloca.into()],
				"",
			)?;

			builder.build_memset(tape_alloca, 1, i8_zero, i8_array_size)?;

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

		let output = {
			let i8_array_type = i8_type.array_type(OUTPUT_ARRAY_LEN.into());
			let i8_array_size = i64_type.const_int(OUTPUT_ARRAY_LEN.into(), false);

			let output_alloca = builder.build_alloca(i8_array_type, "output")?;

			if let Some(output_instr) = output_alloca.as_instruction() {
				output_instr.set_alignment(16)?;
			}

			builder.build_memset(output_alloca, 1, i8_zero, i8_array_size)?;

			output_alloca
		};

		Ok((
			Self {
				tape,
				pointer,
				output,
			},
			ptr_int_type,
		))
	}
}

unsafe impl<'ctx> AsContextRef<'ctx> for AssemblerPointers<'ctx> {
	fn as_ctx_ref(&self) -> LLVMContextRef {
		self.tape.get_type().get_context().as_ctx_ref()
	}
}

pub const OUTPUT_ARRAY_LEN: u32 = 8;
