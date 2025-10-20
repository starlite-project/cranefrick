#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod ext;
mod inner;
mod module;

use std::{
	borrow::Cow,
	error::Error as StdError,
	ffi::CStr,
	fmt::{Debug, Display, Formatter, Result as FmtResult, Write as _},
	fs,
	io::Error as IoError,
	path::{Path, PathBuf},
};

use frick_ir::BrainIr;
use inkwell::{
	OptimizationLevel,
	builder::BuilderError,
	context::Context,
	module::Module,
	passes::PassBuilderOptions,
	support::LLVMString,
	targets::{
		CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
		TargetMachineOptions,
	},
	values::InstructionValueError,
};
use send_wrapper::SendWrapper;
use tracing::info;

pub(crate) use self::ext::*;
pub use self::module::AssembledModule;
use self::{
	inner::{AssemblerFunctions, InnerAssembler},
	module::MemoryManager,
};

pub struct Assembler {
	context: Context,
	passes: String,
	file_path: Option<PathBuf>,
}

impl Assembler {
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

	#[tracing::instrument(skip_all, fields(indicatif.pb_show = tracing::field::Empty))]
	pub fn assemble<'ctx>(
		&'ctx self,
		ops: &[BrainIr],
		output_path: &Path,
	) -> Result<AssembledModule<'ctx>, AssemblyError> {
		info!("initializing all targets");

		Target::initialize_all(&InitializationConfig::default());

		let target_triple = {
			let default_triple = TargetMachine::get_default_triple();

			TargetMachine::normalize_triple(&default_triple)
		};

		let cpu = TargetMachine::get_host_cpu_name().to_string();
		let cpu_features = TargetMachine::get_host_cpu_features().to_string();

		let target = Target::from_triple(&target_triple)?;

		let target_machine = {
			let options = TargetMachineOptions::new()
				.set_cpu(&cpu)
				.set_features(&cpu_features)
				.set_reloc_mode(RelocMode::Default)
				.set_code_model(CodeModel::JITDefault)
				.set_level(OptimizationLevel::Aggressive);

			target
				.create_target_machine_from_options(&target_triple, options)
				.ok_or(AssemblyError::NoTargetMachine)?
		};

		target_machine.set_asm_verbosity(true);

		info!("lowering into LLVM IR");
		let assembler =
			InnerAssembler::new(&self.context, target_machine, self.file_path.as_deref())?;

		let (module, AssemblerFunctions { main, .. }, target_machine) = assembler.assemble(ops)?;

		info!("verifying emitted LLVM IR");
		module.verify()?;

		write_data(
			&target_machine,
			&module,
			output_path,
			ToWriteType::Unoptimized,
		)?;

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

		info!("optimizing LLVM IR");
		module.run_passes(&self.passes, &target_machine, pass_options)?;

		info!("verifying optimized LLVM IR");
		module.verify()?;

		write_data(
			&target_machine,
			&module,
			output_path,
			ToWriteType::Optimized,
		)?;

		if module.strip_debug_info() {
			info!("verifying stripped LLVM IR");
			module.verify()?;

			target_machine.set_asm_verbosity(false);

			write_data(&target_machine, &module, output_path, ToWriteType::Stripped)?;
		}

		info!("creating JIT execution engine");
		let execution_engine = module.create_mcjit_execution_engine_with_memory_manager(
			MemoryManager::new(),
			OptimizationLevel::Aggressive,
			CodeModel::JITDefault,
			false,
			true,
		)?;

		if let Some(getchar) = module.get_function("rust_getchar\0") {
			info!("adding rust_getchar to execution engine");
			execution_engine.add_global_mapping(&getchar, frick_interop::rust_getchar as usize);
		}

		if let Some(putchar) = module.get_function("rust_putchar\0") {
			info!("adding rust_putchar to execution engine");
			execution_engine.add_global_mapping(&putchar, frick_interop::rust_putchar as usize);
		}

		if let Some(eh_personality) = module.get_function("rust_eh_personality\0") {
			info!("adding rust_eh_personality to the execution engine");
			execution_engine
				.add_global_mapping(&eh_personality, frick_interop::rust_eh_personality as usize);
		}

		Ok(AssembledModule {
			execution_engine,
			main,
		})
	}
}

#[cold]
extern "C" fn handler(ptr: *const i8) {
	let c_str = unsafe { CStr::from_ptr(ptr) };

	println!("{}", c_str.to_string_lossy());

	std::process::abort()
}

impl Default for Assembler {
	fn default() -> Self {
		Self::new("default<O0>".to_owned())
	}
}

#[derive(Debug)]
pub enum AssemblyError {
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
	NotImplemented(BrainIr),
	Io(IoError),
}

impl AssemblyError {
	pub(crate) const fn intrinsic_not_found(s: &'static str) -> Self {
		Self::IntrinsicNotFound(Cow::Borrowed(s))
	}

	pub(crate) const fn invalid_intrinsic_declaration(s: &'static str) -> Self {
		Self::InvalidIntrinsicDeclaration(Cow::Borrowed(s))
	}
}

impl Display for AssemblyError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Llvm(..) => f.write_str("an error occurred from LLVM"),
			Self::NoTargetMachine => f.write_str("unable to get target machine"),
			Self::Inkwell(..) => f.write_str("an error occurred during translation"),
			Self::InvalidMetadata => f.write_str("invalid metadata type"),
			Self::IntrinsicNotFound(intrinsic) => {
				f.write_str("intrinsic \"")?;
				f.write_str(intrinsic)?;
				f.write_str("\" was not found")
			}
			Self::InvalidIntrinsicDeclaration(intrinsic) => {
				f.write_str("invalid declaration for intrinsic \"")?;
				f.write_str(intrinsic)?;
				f.write_char('"')
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
			Self::NotImplemented(instr) => {
				f.write_str("instruction ")?;
				Debug::fmt(&instr, f)?;
				f.write_str(" is not yet implemented")
			}
			Self::Io(..) => f.write_str("an IO error has occurred"),
		}
	}
}

impl StdError for AssemblyError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Inkwell(e) => Some(e),
			Self::Llvm(e) => Some(&**e),
			Self::Io(e) => Some(e),
			Self::NoTargetMachine
			| Self::InvalidMetadata
			| Self::IntrinsicNotFound(..)
			| Self::InvalidGEPType(..)
			| Self::InvalidIntrinsicDeclaration(..)
			| Self::NotImplemented(..)
			| Self::MissingPointerInstruction { .. } => None,
		}
	}
}

impl From<LLVMString> for AssemblyError {
	fn from(value: LLVMString) -> Self {
		Self::Llvm(SendWrapper::new(value))
	}
}

impl From<BuilderError> for AssemblyError {
	fn from(value: BuilderError) -> Self {
		Self::Inkwell(value.into())
	}
}

impl From<inkwell::Error> for AssemblyError {
	fn from(value: inkwell::Error) -> Self {
		Self::Inkwell(value)
	}
}

impl From<InstructionValueError> for AssemblyError {
	fn from(value: InstructionValueError) -> Self {
		Self::Inkwell(value.into())
	}
}

impl From<IoError> for AssemblyError {
	fn from(value: IoError) -> Self {
		Self::Io(value)
	}
}

#[tracing::instrument(skip_all, fields(%opt_type))]
fn write_data(
	target_machine: &TargetMachine,
	module: &Module<'_>,
	output_path: &Path,
	opt_type: ToWriteType,
) -> Result<(), AssemblyError> {
	info!("writing LLVM IR");

	{
		let s = module.print_to_string().to_string();
		fs::write(output_path.join(format!("{opt_type}.ll")), s)?;
	}

	info!("writing LLVM bitcode");

	{
		let memory_buf = module.write_bitcode_to_memory();
		fs::write(
			output_path.join(format!("{opt_type}.bc")),
			memory_buf.as_slice(),
		)?;
	}

	info!("writing object file");
	{
		let memory_buffer = target_machine.write_to_memory_buffer(module, FileType::Object)?;
		fs::write(
			output_path.join(format!("{opt_type}.o")),
			memory_buffer.as_slice(),
		)?;
	}

	info!("writing assembly");
	{
		let memory_buffer = target_machine.write_to_memory_buffer(module, FileType::Assembly)?;
		fs::write(
			output_path.join(format!("{opt_type}.s")),
			memory_buffer.as_slice(),
		)?;
	}

	Ok(())
}

#[derive(Debug, Clone, Copy)]
enum ToWriteType {
	Unoptimized,
	Optimized,
	Stripped,
}

impl Display for ToWriteType {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str(match *self {
			Self::Unoptimized => "unoptimized",
			Self::Optimized => "optimized",
			Self::Stripped => "stripped",
		})
	}
}
