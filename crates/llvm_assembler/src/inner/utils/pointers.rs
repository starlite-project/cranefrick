use frick_assembler::TAPE_SIZE;
use inkwell::{
	builder::Builder,
	context::{Context, ContextRef},
	module::Module,
	targets::TargetData,
	types::IntType,
	values::PointerValue,
};

use super::AssemblerFunctions;
use crate::LlvmAssemblyError;

#[derive(Debug, Clone, Copy)]
pub struct AssemblerPointers<'ctx> {
	pub tape: PointerValue<'ctx>,
	pub pointer: PointerValue<'ctx>,
	pub output: PointerValue<'ctx>,
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
		let ptr_int_type = context.ptr_sized_int_type(target_data, None);

		let tape = {
			let i8_array_type = i8_type.array_type(TAPE_SIZE as u32);

			builder.build_alloca(i8_array_type, "tape")?
		};

		let pointer = builder.build_alloca(ptr_int_type, "pointer")?;

		let output = {
			let i8_array_type = i8_type.array_type(8);

			builder.build_alloca(i8_array_type, "output")?
		};

		Self {
			tape,
			pointer,
			output,
		}
		.setup(context, functions, builder, ptr_int_type)
	}

	fn setup(
		self,
		context: ContextRef<'ctx>,
		functions: AssemblerFunctions<'ctx>,
		builder: &Builder<'ctx>,
		ptr_int_type: IntType<'ctx>,
	) -> Result<(Self, IntType<'ctx>), LlvmAssemblyError> {
		let i64_type = context.i64_type();

		let zero_for_cell = {
			let i8_type = context.i8_type();

			i8_type.const_zero()
		};

		let tape_size = i64_type.const_int(TAPE_SIZE as u64, false);

		let ptr_int_size = i64_type.const_int(8, false);

		builder.build_call(
			functions.lifetime.start,
			&[tape_size.into(), self.tape.into()],
			"",
		)?;

		builder.build_memset(self.tape, 1, zero_for_cell, tape_size)?;

		builder.build_call(
			functions.lifetime.start,
			&[ptr_int_size.into(), self.pointer.into()],
			"",
		)?;

		builder.build_store(self.pointer, ptr_int_type.const_zero())?;

		Ok((self, ptr_int_type))
	}
}
