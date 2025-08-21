use frick_assembler::AssemblyError;
use frick_ir::BrainIr;
use inkwell::{
	AddressSpace,
	builder::Builder,
	context::Context,
	module::{Linkage, Module},
	values::{FunctionValue, PointerValue},
};

use super::ContextExt;
use crate::LlvmAssemblyError;

pub struct InnerAssembler<'ctx> {
	pub context: &'ctx Context,
	pub module: Module<'ctx>,
	pub builder: Builder<'ctx>,
	pub functions: Functions<'ctx>,
	tape: PointerValue<'ctx>,
	ptr: PointerValue<'ctx>,
}

impl<'ctx> InnerAssembler<'ctx> {
	pub fn new(context: &'ctx Context) -> Self {
		let module = context.create_module("frick");
		let functions = Functions::new(context, &module);

		let tape = {
			let i8_type = context.i8_type();
			let i8_array_type = i8_type.array_type(30_000);

			let glob = module.add_global(i8_array_type, Some(AddressSpace::default()), "tape");

			let i8_array = i8_array_type.const_zero();

			glob.set_initializer(&i8_array);

			glob.as_pointer_value()
		};

		let ptr = {
			let i64_type = context.i64_type();

			let glob = module.add_global(i64_type, Some(AddressSpace::default()), "ptr");

			let i64_zero = i64_type.const_zero();

			glob.set_initializer(&i64_zero);

			glob.as_pointer_value()
		};

		Self {
			context,
			module,
			builder: context.create_builder(),
			functions,
			tape,
			ptr,
		}
	}

	pub fn assemble(
		self,
		ops: &[BrainIr],
	) -> Result<(Module<'ctx>, Functions<'ctx>), AssemblyError<LlvmAssemblyError>> {
		let basic_block = self
			.context
			.append_basic_block(self.functions.main, "entry");
		self.builder.position_at_end(basic_block);

		let ptr_value = self
			.builder
			.build_load(self.context.i64_type(), self.ptr, "load ptr")
			.map_err(AssemblyError::backend)?
			.into_int_value();

		let tape_value = self
			.builder
			.build_load(
				self.context.i8_type().array_type(30_000),
				self.tape,
				"load tape",
			)
			.map_err(AssemblyError::backend)?;

		dbg!(ptr_value);
		dbg!(tape_value);

		self.builder
			.build_return(None)
			.map_err(AssemblyError::backend)?;
		Ok(self.into_parts())
	}

	fn into_parts(self) -> (Module<'ctx>, Functions<'ctx>) {
		(self.module, self.functions)
	}
}

pub struct Functions<'ctx> {
	pub getchar: FunctionValue<'ctx>,
	pub putchar: FunctionValue<'ctx>,
	pub main: FunctionValue<'ctx>,
}

impl<'ctx> Functions<'ctx> {
	fn new(context: &'ctx Context, module: &Module<'ctx>) -> Self {
		let ptr_type = context.default_ptr_type();
		let i8_type = context.i8_type();
		let void_type = context.void_type();

		let getchar_ty = void_type.fn_type(&[ptr_type.into()], false);
		let getchar = module.add_function("getchar", getchar_ty, Some(Linkage::External));

		let putchar_ty = void_type.fn_type(&[i8_type.into()], false);
		let putchar = module.add_function("putchar", putchar_ty, Some(Linkage::External));

		let main_ty = void_type.fn_type(&[], false);
		let main = module.add_function("main", main_ty, Some(Linkage::External));

		Self {
			getchar,
			putchar,
			main,
		}
	}
}
