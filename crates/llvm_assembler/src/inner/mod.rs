use frick_assembler::AssemblyError;
use frick_ir::BrainIr;
use inkwell::{
	builder::Builder,
	context::Context,
	module::{Linkage, Module},
	values::FunctionValue,
};

use super::ContextExt;
use crate::LlvmAssemblyError;

pub struct InnerAssembler<'ctx> {
	pub context: &'ctx Context,
	pub module: Module<'ctx>,
	pub builder: Builder<'ctx>,
	pub functions: Functions<'ctx>,
}

impl<'ctx> InnerAssembler<'ctx> {
	pub fn new(context: &'ctx Context) -> Self {
		let module = context.create_module("frick");
		let functions = Functions::new(context, &module);

		Self {
			context,
			module,
			builder: context.create_builder(),
			functions,
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

		self.builder.build_return(None).map_err(AssemblyError::backend)?;
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
