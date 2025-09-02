#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod module;

use std::{
	error::Error as StdError,
	fmt::{Display, Formatter, Result as FmtResult},
};

use frick_assembler::{Assembler, InnerAssemblyError};

pub use self::module::RustInterpreterModule;

pub struct RustInterpreterAssembler;

impl Assembler for RustInterpreterAssembler {
	type Error = RustInterpreterError;
	type Module<'ctx>
		= RustInterpreterModule<'ctx>
	where
		Self: 'ctx;

	fn assemble<'ctx>(
		&'ctx self,
		ops: &[frick_ir::BrainIr],
		_: &std::path::Path,
	) -> Result<Self::Module<'ctx>, frick_assembler::AssemblyError<Self::Error>> {
		Ok(RustInterpreterModule::new(ops.to_owned()))
	}
}

#[derive(Debug)]
pub enum RustInterpreterError {}

impl Display for RustInterpreterError {
	fn fmt(&self, _: &mut Formatter<'_>) -> FmtResult {
		Ok(())
	}
}

impl StdError for RustInterpreterError {}

impl InnerAssemblyError for RustInterpreterError {}
