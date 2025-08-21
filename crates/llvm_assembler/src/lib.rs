#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod ext;
mod inner;
mod module;

use std::{
	error::Error as StdError,
	fmt::{Display, Formatter, Result as FmtResult},
	marker::PhantomData,
	path::Path,
};

use frick_assembler::{
	Assembler, AssemblyError, InnerAssemblyError, frick_assembler_read, frick_assembler_write,
};
use frick_ir::BrainIr;
use inkwell::{context::Context, support::LLVMString};
use inner::Functions;

pub(crate) use self::ext::ContextExt;
use self::inner::InnerAssembler;
pub use self::module::LlvmAssembledModule;

pub struct LlvmAssembler {
	context: Context,
}

impl Assembler for LlvmAssembler {
	type Error = LlvmAssemblyError;
	type Module<'ctx> = LlvmAssembledModule<'ctx>;

	fn assemble<'ctx>(
		&'ctx self,
		ops: &[BrainIr],
		output_path: &Path,
	) -> Result<Self::Module<'ctx>, AssemblyError<Self::Error>> {
		let context = &self.context;

		let assembler = InnerAssembler::new(&self.context);

		let (module, builder, Functions { getchar, putchar }) = assembler.into_parts();

		module
			.print_to_file(output_path.join("unoptimized.ir"))
			.map_err(AssemblyError::backend)?;

		let execution_engine = module
			.create_execution_engine()
			.map_err(AssemblyError::backend)?;

		execution_engine.add_global_mapping(&getchar, frick_assembler_read as usize);
		execution_engine.add_global_mapping(&putchar, frick_assembler_write as usize);

		Ok(LlvmAssembledModule {
			context,
			module,
			execution_engine,
		})
	}
}

impl Default for LlvmAssembler {
	fn default() -> Self {
		Self {
			context: Context::create(),
		}
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

impl InnerAssemblyError for LlvmAssemblyError {}
