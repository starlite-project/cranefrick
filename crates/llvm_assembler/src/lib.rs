#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod ext;
mod inner;
mod module;

use std::{
	error::Error as StdError,
	fmt::{Display, Formatter, Result as FmtResult},
	path::Path,
};

use frick_assembler::{
	Assembler, AssemblyError, InnerAssemblyError, frick_assembler_read, frick_assembler_write,
};
use frick_ir::BrainIr;
use inkwell::{
	OptimizationLevel,
	builder::BuilderError,
	context::Context,
	passes::PassBuilderOptions,
	support::LLVMString,
	targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
};
use inner::Functions;

pub(crate) use self::ext::ContextExt;
use self::inner::InnerAssembler;
pub use self::module::LlvmAssembledModule;

pub struct LlvmAssembler {
	context: Context,
	passes: String,
}

impl LlvmAssembler {
	#[must_use]
	pub fn new(passes: String) -> Self {
		Self {
			context: Context::create(),
			passes,
		}
	}
}

impl Assembler for LlvmAssembler {
	type Error = LlvmAssemblyError;
	type Module<'ctx> = LlvmAssembledModule<'ctx>;

	fn assemble<'ctx>(
		&'ctx self,
		ops: &[BrainIr],
		output_path: &Path,
	) -> Result<Self::Module<'ctx>, AssemblyError<Self::Error>> {
		Target::initialize_native(&InitializationConfig::default())
			.map_err(LlvmAssemblyError::Llvm)?;

		let assembler = InnerAssembler::new(&self.context);

		let (module, Functions { main, .. }) = assembler.assemble(ops)?;

		module
			.print_to_file(output_path.join("unoptimized.ir"))
			.map_err(AssemblyError::backend)?;

		{
			let target_triple = TargetMachine::get_default_triple();
			let cpu = TargetMachine::get_host_cpu_name().to_string();
			let features = TargetMachine::get_host_cpu_features().to_string();

			let target = Target::from_triple(&target_triple).map_err(AssemblyError::backend)?;

			let target_machine = target
				.create_target_machine(
					&target_triple,
					&cpu,
					&features,
					OptimizationLevel::Aggressive,
					RelocMode::Default,
					CodeModel::Default,
				)
				.ok_or(LlvmAssemblyError::NoTargetMachine)?;

			let pass_options = PassBuilderOptions::create();

			pass_options.set_verify_each(true);

			module
				.run_passes(&self.passes, &target_machine, pass_options)
				.map_err(AssemblyError::backend)?;
		}

		module
			.print_to_file(output_path.join("optimized.ir"))
			.map_err(AssemblyError::backend)?;

		let execution_engine = module
			.create_execution_engine()
			.map_err(AssemblyError::backend)?;

		if let Some(getchar) = module.get_function("getchar") {
			execution_engine.add_global_mapping(&getchar, frick_assembler_read as usize);
		}

		if let Some(putchar) = module.get_function("putchar") {
			execution_engine.add_global_mapping(&putchar, frick_assembler_write as usize);
		}

		Ok(LlvmAssembledModule {
			execution_engine,
			main,
		})
	}
}

impl Default for LlvmAssembler {
	fn default() -> Self {
		Self::new("default<O0>".to_owned())
	}
}

#[derive(Debug)]
pub enum LlvmAssemblyError {
	Llvm(String),
	NoTargetMachine,
	Builder(BuilderError),
}

impl Display for LlvmAssemblyError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Llvm(l) => {
				f.write_str("an error occurred from LLVM: ")?;
				f.write_str(l)
			}
			Self::NoTargetMachine => f.write_str("unable to get target machine"),
			Self::Builder(..) => f.write_str("an error occurred building an instruction"),
		}
	}
}

impl StdError for LlvmAssemblyError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Builder(e) => Some(e),
			Self::NoTargetMachine | Self::Llvm(..) => None,
		}
	}
}

impl From<LLVMString> for LlvmAssemblyError {
	fn from(value: LLVMString) -> Self {
		Self::Llvm(value.to_string())
	}
}

impl From<BuilderError> for LlvmAssemblyError {
	fn from(value: BuilderError) -> Self {
		Self::Builder(value)
	}
}

impl InnerAssemblyError for LlvmAssemblyError {}
