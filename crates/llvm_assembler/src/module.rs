use frick_assembler::AssembledModule;
use inkwell::{context::Context, execution_engine::ExecutionEngine, module::Module};

use super::LlvmAssemblyError;

pub struct LlvmAssembledModule<'ctx> {
	pub(crate) context: &'ctx Context,
	pub(crate) module: Module<'ctx>,
	pub(crate) execution_engine: ExecutionEngine<'ctx>,
}

impl AssembledModule for LlvmAssembledModule<'_> {
	type Error = LlvmAssemblyError;

	fn execute(&self) -> Result<(), Self::Error> {
		Ok(())
	}
}
