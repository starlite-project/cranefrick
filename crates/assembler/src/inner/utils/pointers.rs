use frick_spec::TAPE_SIZE;
use frick_utils::Convert as _;
use inkwell::{
	builder::Builder,
	context::AsContextRef,
	llvm_sys::prelude::LLVMContextRef,
	module::Module,
	targets::TargetData,
	types::IntType,
	values::{BasicMetadataValueEnum, PointerValue},
};

use super::AssemblerFunctions;
use crate::{AssemblyError, ContextGetter as _};

#[derive(Debug, Clone, Copy)]
pub struct AssemblerPointers<'ctx> {
	pub tape: PointerValue<'ctx>,
	pub pointer: PointerValue<'ctx>,
	pub pointer_ty: IntType<'ctx>,
}

impl<'ctx> AssemblerPointers<'ctx> {
	pub fn new(
		module: &Module<'ctx>,
		builder: &Builder<'ctx>,
		target_data: &TargetData,
	) -> Result<Self, AssemblyError> {
		let context = module.get_context();
		let i8_type = context.i8_type();
		let ptr_int_type = context.ptr_sized_int_type(target_data, None);

		let tape = {
			let tape_array_size = ptr_int_type.const_int(TAPE_SIZE as u64, false);

			builder.build_array_alloca(i8_type, tape_array_size, "tape\0")?
		};

		let pointer = builder.build_alloca(ptr_int_type, "pointer\0")?;

		Ok(Self {
			tape,
			pointer,
			pointer_ty: ptr_int_type,
		})
	}

	pub fn setup(
		self,
		builder: &Builder<'ctx>,
		functions: &AssemblerFunctions<'ctx>,
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i64_type = context.i64_type();

		let tape_array_size = i64_type.const_int(TAPE_SIZE as u64, false);

		builder.build_call(
			functions.lifetime.start,
			&[
				tape_array_size.convert::<BasicMetadataValueEnum<'ctx>>(),
				self.tape.convert::<BasicMetadataValueEnum<'ctx>>(),
			],
			"\0",
		)?;

		let pointer_size = i64_type.const_int(8, false);

		builder.build_call(
			functions.lifetime.start,
			&[
				pointer_size.convert::<BasicMetadataValueEnum<'ctx>>(),
				self.pointer.convert::<BasicMetadataValueEnum<'ctx>>(),
			],
			"\0",
		)?;

		let i8_zero = i8_type.const_zero();

		builder.build_memset(self.tape, 1, i8_zero, tape_array_size)?;
		builder.build_store(self.pointer, self.pointer_ty.const_zero())?;

		Ok(())
	}
}

unsafe impl<'ctx> AsContextRef<'ctx> for AssemblerPointers<'ctx> {
	fn as_ctx_ref(&self) -> LLVMContextRef {
		self.tape.get_type().get_context().as_ctx_ref()
	}
}
