mod memory;

use frick_assembler::AssembledModule;
use inkwell::{execution_engine::ExecutionEngine, values::FunctionValue};

pub use self::memory::*;
use super::LlvmAssemblyError;

pub struct LlvmAssembledModule<'ctx> {
	pub(crate) execution_engine: ExecutionEngine<'ctx>,
	pub(crate) main: FunctionValue<'ctx>,
}

impl AssembledModule for LlvmAssembledModule<'_> {
	type Error = LlvmAssemblyError;

	fn execute(&self) -> Result<(), Self::Error> {
		unsafe { self.execution_engine.run_function_as_main(self.main, &[]) };

		Ok(())
	}
}
