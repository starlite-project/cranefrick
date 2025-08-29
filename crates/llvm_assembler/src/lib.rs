#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod ext;
mod inner;
mod module;

use std::{
	error::Error as StdError,
	ffi::CStr,
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
	targets::{
		CodeModel, InitializationConfig, RelocMode, Target, TargetMachine, TargetMachineOptions,
	},
};
use inner::Functions;
use tracing::info;

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
		inkwell::support::error_handling::reset_fatal_error_handler();
		unsafe {
			inkwell::support::error_handling::install_fatal_error_handler(handler);
		}

		inkwell::support::enable_llvm_pretty_stack_trace();

		Self {
			context: Context::create(),
			passes,
		}
	}
}

extern "C" fn handler(ptr: *const i8) {
	let c_str = unsafe { CStr::from_ptr(ptr) };

	println!("{}", c_str.to_string_lossy());

	std::process::abort()
}

impl Assembler for LlvmAssembler {
	type Error = LlvmAssemblyError;
	type Module<'ctx> = LlvmAssembledModule<'ctx>;

	#[tracing::instrument(skip_all)]
	fn assemble<'ctx>(
		&'ctx self,
		ops: &[BrainIr],
		output_path: &Path,
	) -> Result<Self::Module<'ctx>, AssemblyError<Self::Error>> {
		info!("initializing native target");

		Target::initialize_all(&InitializationConfig::default());

		let target_triple = TargetMachine::get_default_triple();
		let cpu = TargetMachine::get_host_cpu_name().to_string();
		let features = TargetMachine::get_host_cpu_features().to_string();

		let target = Target::from_triple(&target_triple).map_err(AssemblyError::backend)?;

		let target_machine = {
			let options = TargetMachineOptions::new()
				.set_cpu(&cpu)
				.set_features(&features)
				.set_reloc_mode(RelocMode::PIC)
				.set_code_model(CodeModel::JITDefault)
				.set_level(OptimizationLevel::Aggressive);

			target
				.create_target_machine_from_options(&target_triple, options)
				.ok_or(LlvmAssemblyError::NoTargetMachine)?
		};

		target_machine.set_asm_verbosity(true);

		info!("lowering into LLVM IR");

		let assembler = InnerAssembler::new(&self.context)?;

		let (module, Functions { main, .. }) = assembler.assemble(ops)?;

		let data_layout = {
			let target_data = target_machine.get_target_data();

			target_data.get_data_layout()
		};

		module.set_data_layout(&data_layout);
		module.set_triple(&target_triple);

		info!("writing unoptimized LLVM IR");
		module
			.print_to_file(output_path.join("unoptimized.ll"))
			.map_err(AssemblyError::backend)?;

		let pass_options = PassBuilderOptions::create();

		pass_options.set_verify_each(true);
		pass_options.set_loop_interleaving(true);
		pass_options.set_loop_vectorization(true);
		pass_options.set_loop_slp_vectorization(true);
		pass_options.set_loop_unrolling(true);
		pass_options.set_forget_all_scev_in_loop_unroll(true);
		pass_options.set_call_graph_profile(true);
		pass_options.set_merge_functions(true);
		pass_options.set_licm_mssa_opt_cap(u32::MAX);
		pass_options.set_licm_mssa_no_acc_for_promotion_cap(u32::MAX);

		info!("verifying and optimizing LLVM IR");

		module
			.run_passes(&self.passes, &target_machine, pass_options)
			.map_err(AssemblyError::backend)?;

		info!("writing asm");
		target_machine
			.write_to_file(
				&module,
				inkwell::targets::FileType::Assembly,
				&output_path.join("program.asm"),
			)
			.map_err(AssemblyError::backend)?;

		info!("writing optimized LLVM IR");

		module
			.print_to_file(output_path.join("optimized.ll"))
			.map_err(AssemblyError::backend)?;

		module.verify().map_err(AssemblyError::backend)?;

		info!("creating JIT execution engine");

		let execution_engine = module
			.create_jit_execution_engine(OptimizationLevel::Aggressive)
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
	InvalidMetadata,
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
			Self::InvalidMetadata => f.write_str("invalid metadata type"),
		}
	}
}

impl StdError for LlvmAssemblyError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Builder(e) => Some(e),
			Self::NoTargetMachine | Self::Llvm(..) | Self::InvalidMetadata => None,
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
