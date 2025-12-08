use frick_spec::{POINTER_SIZE, TAPE_SIZE};
use frick_utils::Convert as _;
use inkwell::{
	attributes::AttributeLoc,
	builder::Builder,
	context::AsContextRef,
	llvm_sys::prelude::LLVMContextRef,
	module::Module,
	values::{BasicMetadataValueEnum, FunctionValue, PointerValue},
};

use super::AssemblerFunctions;
use crate::{AssemblyError, ContextExt, ContextGetter as _};

#[derive(Debug, Clone, Copy)]
pub struct AssemblerPointers<'ctx> {
	pub tape: PointerValue<'ctx>,
	pub pointer: PointerValue<'ctx>,
}

impl<'ctx> AssemblerPointers<'ctx> {
	pub fn new(
		module: &Module<'ctx>,
		builder: &Builder<'ctx>,
		malloc: FunctionValue<'ctx>,
	) -> Result<Self, AssemblyError> {
		let context = module.get_context();
		let ptr_int_type = context.custom_width_int_type(POINTER_SIZE as u32);

		let tape = {
			let tape_size = ptr_int_type.const_int(TAPE_SIZE as u64, false);

			let tape_malloc_call = builder.build_call(
				malloc,
				&[tape_size.convert::<BasicMetadataValueEnum<'ctx>>()],
				"tape",
			)?;

			let dereferenceable_or_null_attr =
				context.create_named_enum_attribute("dereferenceable_or_null", TAPE_SIZE as u64);

			tape_malloc_call.add_attribute(AttributeLoc::Return, dereferenceable_or_null_attr);

			tape_malloc_call
				.try_as_basic_value()
				.expect_basic("expected ptr from malloc")
				.into_pointer_value()
		};

		let pointer = builder.build_alloca(ptr_int_type, "pointer\0")?;

		Ok(Self { tape, pointer })
	}

	pub fn setup(
		self,
		builder: &Builder<'ctx>,
		functions: AssemblerFunctions<'ctx>,
	) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i64_type = context.i64_type();
		let ptr_type = context.custom_width_int_type(POINTER_SIZE as u32);

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
		builder.build_store(self.pointer, ptr_type.const_zero())?;

		Ok(())
	}
}

unsafe impl<'ctx> AsContextRef<'ctx> for AssemblerPointers<'ctx> {
	fn as_ctx_ref(&self) -> LLVMContextRef {
		self.tape.get_type().get_context().as_ctx_ref()
	}
}
