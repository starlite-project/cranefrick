#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod flags;
mod inner;
mod module;

use std::{
	error::Error as StdError,
	fmt::{Display, Formatter, Result as FmtResult},
	fs,
	marker::PhantomData,
	path::Path,
};

use cranelift_codegen::{
	CodegenError, CompileError, cfg_printer::CFGPrinter, control::ControlPlane, isa, settings,
};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module as _, ModuleError};
use frick_assembler::{
	Assembler, AssemblyError, InnerAssemblyError, frick_assembler_read, frick_assembler_write,
};
use frick_ir::BrainIr;
use inner::InnerAssembler;
use target_lexicon::Triple;
use tracing::info;

pub use self::{flags::AssemblerFlags, module::CraneliftAssembledModule};

#[derive(Debug)]
pub struct CraneliftAssembler {
	flags: AssemblerFlags,
}

impl CraneliftAssembler {
	#[must_use]
	pub fn new() -> Self {
		Self::with_flags(AssemblerFlags::default())
	}

	#[must_use]
	pub const fn with_flags(flags: AssemblerFlags) -> Self {
		Self { flags }
	}

	pub const fn set_flags(&mut self, flags: AssemblerFlags) {
		self.flags = flags;
	}
}

impl Assembler for CraneliftAssembler {
	type Error = CraneliftAssemblyError;
	type Module<'ctx> = CraneliftAssembledModule<'ctx>;

	#[tracing::instrument(skip_all)]
	fn assemble<'ctx>(
		&'ctx self,
		ops: &[BrainIr],
		output_path: &Path,
	) -> Result<Self::Module<'ctx>, AssemblyError<Self::Error>> {
		let triple = Triple::host();

		info!("looking up ISA");

		let isa = {
			let flags = self.flags.try_into().map_err(AssemblyError::backend)?;

			isa::lookup(triple)
				.map_err(AssemblyError::backend)?
				.finish(flags)
				.map_err(AssemblyError::backend)
		}?;

		info!("creating JIT module");

		let mut jit_builder =
			JITBuilder::with_isa(isa.clone(), cranelift_module::default_libcall_names());

		jit_builder.symbol("write", frick_assembler_write as *const u8);
		jit_builder.symbol("read", frick_assembler_read as *const u8);

		let mut module = JITModule::new(jit_builder);

		info!("declaring main function");

		let ptr_type = module.target_config().pointer_type();

		let mut ctx = module.make_context();
		ctx.func.collect_debug_info();

		let mut fn_ctx = FunctionBuilderContext::new();

		let sig = module.make_signature();

		let func = module
			.declare_function("main", Linkage::Local, &sig)
			.map_err(AssemblyError::backend)?;

		ctx.func.signature = sig;

		info!("lowering into cranelift IR");

		let inner = InnerAssembler::new(&mut ctx.func, &mut fn_ctx, &mut module, ptr_type)?;

		inner.assemble(ops)?;

		{
			info!("writing unoptimized cranelift IR");
			fs::write(output_path.join("unoptimized.clif"), ctx.func.to_string())?;

			info!("writing unoptimized CFG dot graph");
			let mut out = String::new();

			let printer = CFGPrinter::new(&ctx.func);

			printer.write(&mut out)?;

			fs::write(output_path.join("unoptimized.dot"), out)?;
		}

		info!("verifying cranelift IR");
		ctx.verify(&*isa).unwrap();

		info!("optimizing cranelift IR");
		ctx.optimize(&*isa, &mut ControlPlane::default())
			.map_err(AssemblyError::backend)?;

		{
			info!("writing optimized cranelift IR");
			fs::write(output_path.join("optimized.clif"), ctx.func.to_string())?;

			info!("writing optimized CFG dot graph");
			let mut out = String::new();

			let printer = CFGPrinter::new(&ctx.func);

			printer.write(&mut out)?;

			fs::write(output_path.join("optimized.dot"), out)?;
		}

		{
			info!("compiling binary");
			let compiled_func = ctx
				.compile(&*isa, &mut ControlPlane::default())
				.map_err(AssemblyError::backend)?;

			info!("writing compiled binary");
			fs::write(output_path.join("program.bin"), compiled_func.code_buffer())?;
		}

		info!("finishing up module definitions");
		module
			.define_function(func, &mut ctx)
			.map_err(AssemblyError::backend)?;
		module.clear_context(&mut ctx);

		info!("lowering to native assembly");
		module
			.finalize_definitions()
			.map_err(AssemblyError::backend)?;

		Ok(CraneliftAssembledModule {
			module: Some(module),
			main: func,
			marker: PhantomData,
		})
	}
}

impl Default for CraneliftAssembler {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug)]
pub enum CraneliftAssemblyError {
	NoModuleFound,
	Codegen(CodegenError),
	Module(ModuleError),
	Lookup(isa::LookupError),
}

impl Display for CraneliftAssemblyError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Codegen(..) => f.write_str("a codegen error occurred"),
			Self::Module(..) => f.write_str("a module error occurred"),
			Self::Lookup(..) => f.write_str("an error occurred during ISA lookup"),
			Self::NoModuleFound => f.write_str("module was not assembled"),
		}
	}
}

impl StdError for CraneliftAssemblyError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Codegen(e) => Some(e),
			Self::Module(e) => Some(e),
			Self::Lookup(e) => Some(e),
			Self::NoModuleFound => None,
		}
	}
}

impl From<settings::SetError> for CraneliftAssemblyError {
	fn from(value: settings::SetError) -> Self {
		Self::Module(value.into())
	}
}

impl<'a> From<CompileError<'a>> for CraneliftAssemblyError {
	fn from(value: CompileError<'a>) -> Self {
		Self::from(value.inner)
	}
}

impl From<CodegenError> for CraneliftAssemblyError {
	fn from(value: CodegenError) -> Self {
		Self::Codegen(value)
	}
}

impl From<ModuleError> for CraneliftAssemblyError {
	fn from(value: ModuleError) -> Self {
		Self::Module(value)
	}
}

impl From<isa::LookupError> for CraneliftAssemblyError {
	fn from(value: isa::LookupError) -> Self {
		Self::Lookup(value)
	}
}

impl InnerAssemblyError for CraneliftAssemblyError {}
