use frick_assembler::AssembledModule;

use super::LlvmAssemblyError;

pub struct LlvmAssembledModule;

impl AssembledModule for LlvmAssembledModule {
	type Error = LlvmAssemblyError;

	fn execute(&self) -> Result<(), Self::Error> {
		Ok(())
	}
}
