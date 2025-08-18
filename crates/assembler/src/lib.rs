#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod assembler;
mod flags;

use std::{
	error::Error as StdError,
	fmt::{Debug, Display, Error as FmtError, Formatter, Result as FmtResult},
	fs,
	io::{self, Error as IoError, prelude::*},
	path::Path,
	process::exit,
	slice,
};

use cranefrick_mlir::{BrainMlir, Compiler};
use cranelift_codegen::{
	CodegenError, CompileError, cfg_printer::CFGPrinter, control::ControlPlane, ir::AbiParam, isa,
	settings,
};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module as _, ModuleError};
use target_lexicon::Triple;
use tracing::{Span, error, info, info_span};
use tracing_indicatif::{span_ext::IndicatifSpanExt as _, style::ProgressStyle};

use self::assembler::Assembler;
pub use self::flags::*;

pub struct AssembledModule {
	module: Option<JITModule>,
	main: FuncId,
}

impl AssembledModule {
	#[tracing::instrument(skip_all)]
	pub fn assemble(
		compiler: Compiler,
		flags: AssemblerFlags,
		output_path: &Path,
	) -> Result<Self, AssemblyError> {
		let assemble_span = Span::current();
		assemble_span.pb_set_style(
			&ProgressStyle::with_template(
				"{span_child_prefix}{spinner} {span_name}({span_fields}) [{elapsed_precise}] [{bar:13}]",
			)
			.unwrap()
			.progress_chars("#>-"),
		);
		assemble_span.pb_set_length(13);

		info!("looking up ISA");

		let triple = Triple::host();

		let isa = isa::lookup(triple)?.finish(flags.try_into()?)?;

		assemble_span.pb_inc(1);
		info!("creating JIT module");

		let mut jit_builder =
			JITBuilder::with_isa(isa.clone(), cranelift_module::default_libcall_names());

		jit_builder.symbol("write", write as *const u8);
		jit_builder.symbol("read", read as *const u8);

		let mut module = JITModule::new(jit_builder);

		assemble_span.pb_inc(1);
		info!("declaring main function");

		let ptr_type = module.target_config().pointer_type();

		let mut ctx = module.make_context();
		ctx.func.collect_debug_info();

		let mut func_ctx = FunctionBuilderContext::new();

		let mut sig = module.make_signature();
		sig.params.push(AbiParam::new(ptr_type));

		let func = module.declare_function("main", Linkage::Local, &sig)?;

		ctx.func.signature = sig;

		assemble_span.pb_inc(1);
		info!("lowering into cranelift IR");

		Assembler::new(&mut ctx.func, &mut func_ctx, &mut module, ptr_type)?.assemble(compiler)?;

		assemble_span.pb_inc(1);
		let writing_files_span = info_span!("writing files");

		writing_files_span.pb_set_style(
			&ProgressStyle::with_template(
				"{span_child_prefix}{spinner} {span_name}({span_fields}) [{elapsed_precise}] [{bar:5}]",
			)
			.unwrap()
			.progress_chars("#>-"),
		);
		writing_files_span.pb_set_length(5);

		writing_files_span.in_scope(|| {
			info!("writing unoptimized cranelift-IR");
			fs::write(output_path.join("unoptimized.clif"), ctx.func.to_string())?;
			writing_files_span.pb_inc(1);
			assemble_span.pb_inc(1);

			info!("writing unoptimized CFG dot graph");
			let mut out = String::new();

			let printer = CFGPrinter::new(&ctx.func);

			printer.write(&mut out)?;
			fs::write(output_path.join("unoptimized.dot"), out)?;

			writing_files_span.pb_inc(1);
			assemble_span.pb_inc(1);

			Ok::<(), AssemblyError>(())
		})?;

		info!("running cranelift optimizations");
		ctx.verify(&*isa).unwrap();
		ctx.optimize(&*isa, &mut ControlPlane::default())?;
		ctx.verify(&*isa).unwrap();
		assemble_span.pb_inc(1);

		writing_files_span.in_scope(|| {
			info!("writing optimized cranelift-IR");
			fs::write(output_path.join("optimized.clif"), ctx.func.to_string())?;
			writing_files_span.pb_inc(1);
			assemble_span.pb_inc(1);

			info!("writing optimized CFG dot graph");
			let mut out = String::new();

			let printer = CFGPrinter::new(&ctx.func);

			printer.write(&mut out)?;
			fs::write(output_path.join("optimized.dot"), out)?;

			writing_files_span.pb_inc(1);
			assemble_span.pb_inc(1);

			Ok::<(), AssemblyError>(())
		})?;

		info!("compiling binary");
		let compiled_func = ctx.compile(&*isa, &mut ControlPlane::default())?;
		assemble_span.pb_inc(1);

		writing_files_span.in_scope(|| {
			info!("writing compiled binary");
			fs::write(output_path.join("program.bin"), compiled_func.code_buffer())?;
			writing_files_span.pb_inc(1);
			assemble_span.pb_inc(1);

			Ok::<(), IoError>(())
		})?;

		drop(writing_files_span);

		info!("finishing up module definitions");
		module.define_function(func, &mut ctx)?;
		module.clear_context(&mut ctx);

		assemble_span.pb_inc(1);
		info!(target = %isa.triple(), "lowering to native assembly");
		module.finalize_definitions()?;

		assemble_span.pb_inc(1);

		Ok(Self {
			module: Some(module),
			main: func,
		})
	}

	pub fn execute(self) -> Result<(), AssemblyError> {
		let module = self.module.as_ref().ok_or(AssemblyError::NoModuleFound)?;

		let code = module.get_finalized_function(self.main);

		let exec = unsafe { std::mem::transmute::<*const u8, fn(*mut u8)>(code) };

		let mut tape = [0u8; 30_000];

		exec(tape.as_mut_ptr());

		Ok(())
	}
}

impl Debug for AssembledModule {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_struct("AssembledModule").finish_non_exhaustive()
	}
}

impl Drop for AssembledModule {
	fn drop(&mut self) {
		if let Some(module) = self.module.take() {
			unsafe {
				module.free_memory();
			}
		}
	}
}

#[derive(Debug)]
pub enum AssemblyError {
	NoModuleFound,
	Io(IoError),
	Codegen(CodegenError),
	Module(Box<ModuleError>),
	Custom(&'static str),
	NotImplemented(BrainMlir),
	Fmt(FmtError),
	Lookup(isa::LookupError),
}

impl Display for AssemblyError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::NoModuleFound => f.write_str("module was not assembled"),
			Self::NotImplemented(i) => {
				f.write_str("instruction ")?;
				Debug::fmt(&i, f)?;
				f.write_str(" is not yet implemented")
			}
			Self::Io(_) => f.write_str("an IO error occurred"),
			Self::Codegen(_) => f.write_str("a codegen error occurred"),
			Self::Module(..) => f.write_str("a module error occurred"),
			Self::Custom(s) => f.write_str(s),
			Self::Fmt(..) => f.write_str("an error occurred during formatting"),
			Self::Lookup(..) => f.write_str("an error occurred during ISA lookup"),
		}
	}
}

impl StdError for AssemblyError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Io(e) => Some(e),
			Self::Codegen(e) => Some(e),
			Self::Module(e) => Some(e),
			Self::Fmt(e) => Some(e),
			Self::Lookup(e) => Some(e),
			Self::NoModuleFound | Self::Custom(..) | Self::NotImplemented(..) => None,
		}
	}
}

impl From<IoError> for AssemblyError {
	fn from(value: IoError) -> Self {
		Self::Io(value)
	}
}

impl From<settings::SetError> for AssemblyError {
	fn from(value: settings::SetError) -> Self {
		Self::Module(Box::new(value.into()))
	}
}

impl From<CodegenError> for AssemblyError {
	fn from(value: CodegenError) -> Self {
		Self::Codegen(value)
	}
}

impl<'a> From<CompileError<'a>> for AssemblyError {
	fn from(value: CompileError<'a>) -> Self {
		Self::from(value.inner)
	}
}

impl From<ModuleError> for AssemblyError {
	fn from(value: ModuleError) -> Self {
		Self::Module(Box::new(value))
	}
}

impl From<FmtError> for AssemblyError {
	fn from(value: FmtError) -> Self {
		Self::Fmt(value)
	}
}

impl From<isa::LookupError> for AssemblyError {
	fn from(value: isa::LookupError) -> Self {
		Self::Lookup(value)
	}
}

unsafe extern "C" fn write(value: u8) {
	if cfg!(target_os = "windows") && value >= 128 {
		return;
	}

	let mut stdout = io::stdout().lock();

	let result = stdout.write_all(&[value]).and_then(|()| stdout.flush());

	if let Err(e) = result {
		error!("error occurred during write: {e}");
		exit(1);
	}
}

unsafe extern "C" fn read(buf: *mut u8) {
	let mut stdin = io::stdin().lock();
	loop {
		let mut value = 0;
		let err = stdin.read_exact(slice::from_mut(&mut value));

		if let Err(e) = err {
			if !matches!(e.kind(), io::ErrorKind::UnexpectedEof) {
				// return Box::into_raw(Box::new(e));
				error!("error occurred during read: {e}");
				exit(1);
			}

			value = 0;
		}

		if cfg!(target_os = "windows") && matches!(value, b'\r') {
			continue;
		}

		unsafe { *buf = value };

		break;
	}
}
