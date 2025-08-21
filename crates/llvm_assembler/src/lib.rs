#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod inner;
mod module;

use std::{
	error::Error as StdError,
	fmt::{Display, Formatter, Result as FmtResult},
	path::Path,
};

use frick_assembler::{Assembler, AssemblyError};
use frick_ir::BrainIr;
use inkwell::{context::Context, support::LLVMString};

use self::inner::InnerAssembler;
pub use self::module::LlvmAssembledModule;

pub struct LlvmAssembler;

impl Assembler for LlvmAssembler {
	type Error = LlvmAssemblyError;
	type Module = LlvmAssembledModule;

	fn assemble(
		&self,
		ops: &[BrainIr],
		output_path: &Path,
	) -> Result<Self::Module, AssemblyError<Self::Error>> {
		let context = Context::create();

		let assembler = InnerAssembler::new(&context)?;

		todo!()
	}
}

impl Default for LlvmAssembler {
	fn default() -> Self {
		Self
	}
}

#[derive(Debug)]
pub enum LlvmAssemblyError {
	Llvm(String),
}

impl Display for LlvmAssemblyError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Llvm(l) => {
				f.write_str("an error occurred from LLVM: ")?;
				f.write_str(l)
			}
		}
	}
}

impl StdError for LlvmAssemblyError {}

impl From<LLVMString> for LlvmAssemblyError {
	fn from(value: LLVMString) -> Self {
		Self::Llvm(value.to_string())
	}
}

impl From<LlvmAssemblyError> for AssemblyError<LlvmAssemblyError> {
	fn from(value: LlvmAssemblyError) -> Self {
		Self::Backend(value)
	}
}
