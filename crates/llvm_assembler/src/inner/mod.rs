use inkwell::{
	builder::Builder, context::Context, execution_engine::ExecutionEngine, module::Module,
};

use super::LlvmAssemblyError;

pub struct InnerAssembler<'ctx> {
	pub context: &'ctx Context,
	pub module: Module<'ctx>,
	pub builder: Builder<'ctx>,
	pub execution_engine: ExecutionEngine<'ctx>,
}

impl<'ctx> InnerAssembler<'ctx> {
	pub fn new(context: &'ctx Context) -> Result<Self, LlvmAssemblyError> {
		let module = context.create_module("frick");
		let execution_engine = module.create_execution_engine()?;

		Ok(Self {
			context,
			module,
			builder: context.create_builder(),
			execution_engine,
		})
	}
}
