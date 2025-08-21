use std::{marker::PhantomData, mem};

use cranelift_jit::JITModule;
use cranelift_module::FuncId;
use frick_assembler::AssembledModule;

use super::CraneliftAssemblyError;

pub struct CraneliftAssembledModule<'ctx> {
	pub(crate) module: Option<JITModule>,
	pub(crate) main: FuncId,
	pub(crate) marker: PhantomData<&'ctx ()>
}

impl<'ctx> AssembledModule for CraneliftAssembledModule<'ctx> {
	type Error = CraneliftAssemblyError;

	fn execute(&self) -> Result<(), Self::Error> {
		let module = self
			.module
			.as_ref()
			.ok_or(CraneliftAssemblyError::NoModuleFound)?;

		let code = module.get_finalized_function(self.main);

		let exec = unsafe { mem::transmute::<*const u8, fn()>(code) };

		exec();

		Ok(())
	}
}

impl Drop for CraneliftAssembledModule<'_> {
	fn drop(&mut self) {
		if let Some(module) = self.module.take() {
			unsafe {
				module.free_memory();
			}
		}
	}
}
