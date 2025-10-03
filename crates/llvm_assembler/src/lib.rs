#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod ext;
mod inner;
mod module;

use std::{
	borrow::Cow,
	error::Error as StdError,
	ffi::CStr,
	fmt::{Display, Formatter, Result as FmtResult, Write as _},
	io::{self, prelude::*},
	path::{Path, PathBuf},
	slice,
};

use frick_assembler::{Assembler, AssemblyError, InnerAssemblyError};
use frick_ir::BrainIr;
use inkwell::{
	OptimizationLevel,
	builder::BuilderError,
	context::Context,
	passes::PassBuilderOptions,
	support::LLVMString,
	targets::{
		CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
		TargetMachineOptions,
	},
	values::InstructionValueError,
};
use inner::AssemblerFunctions;
use send_wrapper::SendWrapper;
use tracing::info;

pub(crate) use self::ext::ContextExt;
use self::inner::InnerAssembler;
pub use self::module::LlvmAssembledModule;

pub struct LlvmAssembler {
	context: Context,
	passes: String,
	file_path: Option<PathBuf>,
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
			file_path: None,
		}
	}

	pub fn set_path(&mut self, path: PathBuf) {
		self.file_path = Some(path);
	}

	#[must_use]
	pub fn with_path(passes: String, path: PathBuf) -> Self {
		let mut this = Self::new(passes);

		this.set_path(path);

		this
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

		let target_triple = {
			let default_triple = TargetMachine::get_default_triple();

			TargetMachine::normalize_triple(&default_triple)
		};
		let cpu = TargetMachine::get_host_cpu_name().to_string();
		let features = TargetMachine::get_host_cpu_features().to_string();

		let target = Target::from_triple(&target_triple).map_err(AssemblyError::backend)?;

		let target_machine = {
			let options = TargetMachineOptions::new()
				.set_cpu(&cpu)
				.set_features(&features)
				.set_reloc_mode(RelocMode::Static)
				.set_code_model(CodeModel::Default)
				.set_level(OptimizationLevel::Aggressive);

			target
				.create_target_machine_from_options(&target_triple, options)
				.ok_or(LlvmAssemblyError::NoTargetMachine)?
		};

		target_machine.set_asm_verbosity(true);

		info!("lowering into LLVM IR");

		let assembler =
			InnerAssembler::new(&self.context, target_machine, self.file_path.as_deref())?;

		let (module, AssemblerFunctions { main, .. }, target_machine) = assembler.assemble(ops)?;

		info!("writing unoptimized LLVM IR");
		module
			.print_to_file(output_path.join("unoptimized.ll"))
			.map_err(AssemblyError::backend)?;

		info!("writing unoptimized object file");
		target_machine
			.write_to_file(
				&module,
				FileType::Object,
				&output_path.join("unoptimized.o"),
			)
			.map_err(AssemblyError::backend)?;

		info!("writing unoptimized asm");
		target_machine
			.write_to_file(
				&module,
				FileType::Assembly,
				&output_path.join("unoptimized.asm"),
			)
			.map_err(AssemblyError::backend)?;

		info!("writing unoptimized LLVM bitcode");
		module.write_bitcode_to_path(output_path.join("unoptimized.bc"));

		let pass_options = PassBuilderOptions::create();

		pass_options.set_verify_each(false);
		pass_options.set_loop_interleaving(true);
		pass_options.set_loop_vectorization(true);
		pass_options.set_loop_slp_vectorization(true);
		pass_options.set_loop_unrolling(true);
		pass_options.set_forget_all_scev_in_loop_unroll(true);
		pass_options.set_call_graph_profile(true);
		pass_options.set_merge_functions(true);
		pass_options.set_licm_mssa_opt_cap(u32::MAX);
		pass_options.set_licm_mssa_no_acc_for_promotion_cap(u32::MAX);

		info!("verifying LLVM IR");

		module.verify().map_err(AssemblyError::backend)?;

		info!("optimizing LLVM IR");

		module
			.run_passes(&self.passes, &target_machine, pass_options)
			.map_err(AssemblyError::backend)?;

		info!("writing optimized LLVM IR");
		module
			.print_to_file(output_path.join("optimized.ll"))
			.map_err(AssemblyError::backend)?;

		info!("writing optimized object file");
		target_machine
			.write_to_file(&module, FileType::Object, &output_path.join("optimized.o"))
			.map_err(AssemblyError::backend)?;

		info!("writing optimized asm");
		target_machine
			.write_to_file(
				&module,
				FileType::Assembly,
				&output_path.join("optimized.asm"),
			)
			.map_err(AssemblyError::backend)?;

		info!("writing optimized LLVM bitcode");
		module.write_bitcode_to_path(output_path.join("optimized.bc"));

		module.verify().map_err(AssemblyError::backend)?;

		info!("creating JIT execution engine");

		let execution_engine = module
			.create_jit_execution_engine(OptimizationLevel::Aggressive)
			.map_err(AssemblyError::backend)?;

		if let Some(getchar) = module.get_function("getchar") {
			execution_engine.add_global_mapping(&getchar, libc::getchar as usize);
		}

		if let Some(putchar) = module.get_function("putchar") {
			execution_engine.add_global_mapping(&putchar, frick_llvm_interop::putchar as usize);
		}

		if let Some(eh_personality) = module.get_function("eh_personality") {
			execution_engine
				.add_global_mapping(&eh_personality, frick_llvm_interop::eh_personality as usize);
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
	Llvm(SendWrapper<LLVMString>),
	NoTargetMachine,
	InvalidMetadata,
	IntrinsicNotFound(Cow<'static, str>),
	InvalidIntrinsicDeclaration(Cow<'static, str>),
	InvalidGEPType(String),
	Inkwell(inkwell::Error),
	MissingPointerInstruction {
		alloca_name: String,
		looking_after: bool,
	},
}

impl LlvmAssemblyError {
	pub(crate) const fn intrinsic_not_found(s: &'static str) -> Self {
		Self::IntrinsicNotFound(Cow::Borrowed(s))
	}

	pub(crate) const fn invalid_intrinsic_declaration(s: &'static str) -> Self {
		Self::InvalidIntrinsicDeclaration(Cow::Borrowed(s))
	}
}

impl Display for LlvmAssemblyError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Llvm(..) => f.write_str("an error occurred from LLVM"),
			Self::NoTargetMachine => f.write_str("unable to get target machine"),
			Self::Inkwell(..) => f.write_str("an error occurred during translation"),
			Self::InvalidMetadata => f.write_str("invalid metadata type"),
			Self::IntrinsicNotFound(intrinsic) => {
				f.write_str("instrinsic '")?;
				f.write_str(intrinsic)?;
				f.write_str("' was not found")
			}
			Self::InvalidIntrinsicDeclaration(intrinsic) => {
				f.write_str("invalid declaration for intrinsic '")?;
				f.write_str(intrinsic)?;
				f.write_char('\'')
			}
			Self::InvalidGEPType(ty) => {
				f.write_str("type ")?;
				f.write_str(ty)?;
				f.write_str(" is invalid for GEP")
			}
			Self::MissingPointerInstruction {
				alloca_name,
				looking_after: false,
			} => {
				f.write_str("alloca for '")?;
				f.write_str(alloca_name)?;
				f.write_str("' could not be found")
			}
			Self::MissingPointerInstruction {
				alloca_name,
				looking_after: true,
			} => {
				f.write_str("instruction after alloca '")?;
				f.write_str(alloca_name)?;
				f.write_str("' was not found")
			}
		}
	}
}

impl StdError for LlvmAssemblyError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Inkwell(e) => Some(e),
			Self::Llvm(e) => Some(&**e),
			Self::NoTargetMachine
			| Self::InvalidMetadata
			| Self::IntrinsicNotFound(..)
			| Self::InvalidGEPType(..)
			| Self::InvalidIntrinsicDeclaration(..)
			| Self::MissingPointerInstruction { .. } => None,
		}
	}
}

impl From<LLVMString> for LlvmAssemblyError {
	fn from(value: LLVMString) -> Self {
		Self::Llvm(SendWrapper::new(value))
	}
}

impl From<BuilderError> for LlvmAssemblyError {
	fn from(value: BuilderError) -> Self {
		Self::Inkwell(value.into())
	}
}

impl From<inkwell::Error> for LlvmAssemblyError {
	fn from(value: inkwell::Error) -> Self {
		Self::Inkwell(value)
	}
}

impl From<InstructionValueError> for LlvmAssemblyError {
	fn from(value: InstructionValueError) -> Self {
		Self::Inkwell(value.into())
	}
}

impl InnerAssemblyError for LlvmAssemblyError {}
