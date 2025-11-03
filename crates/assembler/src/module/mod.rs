mod memory;

use inkwell::{execution_engine::ExecutionEngine, values::FunctionValue};

pub use self::memory::*;
use super::AssemblyError;

pub struct AssembledModule<'ctx> {
	pub(crate) execution_engine: ExecutionEngine<'ctx>,
	pub(crate) main: FunctionValue<'ctx>,
}

impl AssembledModule<'_> {
	pub fn execute(&self) -> Result<(), AssemblyError> {
		unsafe {
			self.execution_engine.run_function_as_main(self.main, &[]);
		}

		Ok(())
	}
}

impl Drop for AssembledModule<'_> {
	fn drop(&mut self) {
		self.execution_engine.free_fn_machine_code(self.main);
	}
}
