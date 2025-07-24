#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod flags;

use std::{
	error::Error as StdError,
	fmt::{Debug, Display, Error as FmtError, Formatter, Result as FmtResult},
	fs,
	io::{self, Error as IoError, prelude::*},
	num::NonZero,
	ops::{Deref, DerefMut},
	path::Path,
	ptr, slice,
};

use cranefrick_mlir::{BrainMlir, Compiler};
use cranelift_codegen::{
	CodegenError,
	cfg_printer::CFGPrinter,
	control::ControlPlane,
	ir::{AbiParam, Block, FuncRef, Function, InstBuilder as _, MemFlags, Type, Value, types},
	isa, settings,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, FuncId, Linkage, Module as _, ModuleError};
use target_lexicon::Triple;
use tracing::{info, info_span};

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
		let triple = Triple::host();

		let isa = isa::lookup(triple)?.finish(flags.try_into()?)?;
		info!("lowering to cranelift IR");

		let mut jit_builder =
			JITBuilder::with_isa(isa.clone(), cranelift_module::default_libcall_names());

		jit_builder.symbol("write", write as *const u8);
		jit_builder.symbol("read", read as *const u8);

		let mut module = JITModule::new(jit_builder);
		let ptr_type = module.target_config().pointer_type();

		let mut ctx = module.make_context();
		ctx.func.collect_debug_info();

		let mut func_ctx = FunctionBuilderContext::new();

		let mut sig = module.make_signature();

		sig.returns.push(AbiParam::new(ptr_type));

		let func = module.declare_function("main", Linkage::Local, &sig)?;

		ctx.func.signature = sig;

		Assembler::new(&mut ctx.func, &mut func_ctx, &mut module, ptr_type)?.assemble(compiler)?;

		let span = info_span!("writing files");

		span.in_scope(|| {
			info!("writing unoptimized cranelift-IR");
			fs::write(output_path.join("unoptimized.clif"), ctx.func.to_string())
		})?;

		info!("running cranelift optimizations");
		ctx.verify(&*isa).unwrap();
		ctx.optimize(&*isa, &mut ControlPlane::default())?;
		ctx.verify(&*isa).unwrap();

		span.in_scope(|| {
			info!("writing optimized cranelift-IR");
			fs::write(output_path.join("optimized.clif"), ctx.func.to_string())
		})?;

		let compiled_func = ctx.compile(&*isa, &mut ControlPlane::default()).unwrap();

		span.in_scope(|| {
			info!("writing compiled binary");
			fs::write(output_path.join("program.bin"), compiled_func.code_buffer())
		})?;

		span.in_scope(|| {
			info!("writing CFG dot graph");

			let mut out = String::new();

			let printer = CFGPrinter::new(&ctx.func);

			printer.write(&mut out)?;

			fs::write(output_path.join("program.dot"), out)?;

			Ok::<(), AssemblyError>(())
		})?;

		drop(span);

		info_span!("loop analysis").in_scope(|| {
			info!("performing loop analysis");

			let mut loop_analyzer = cranelift_codegen::loop_analysis::LoopAnalysis::new();

			loop_analyzer.compute(&ctx.func, &ctx.cfg, &ctx.domtree);
		});

		info!("finishing up module definitions");
		module.define_function(func, &mut ctx)?;
		module.clear_context(&mut ctx);

		info!(target = %isa.triple(), "lowering to native assembly");
		module.finalize_definitions()?;

		Ok(Self {
			module: Some(module),
			main: func,
		})
	}

	pub fn execute(self) -> Result<(), AssemblyError> {
		let module = self.module.as_ref().ok_or(AssemblyError::NoModuleFound)?;

		let code = module.get_finalized_function(self.main);

		let exec = unsafe { std::mem::transmute::<*const u8, fn() -> *mut ()>(code) };

		let ptr = exec();

		if !ptr.is_null() {
			let err = unsafe { Box::<IoError>::from_raw(ptr.cast::<IoError>()) };

			return Err((*err).into());
		}

		Ok(())
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

struct Assembler<'a> {
	ptr_type: Type,
	builder: FunctionBuilder<'a>,
	read: FuncRef,
	write: FuncRef,
	exit_block: Block,
	memory_address: Value,
}

impl<'a> Assembler<'a> {
	fn new(
		func: &'a mut Function,
		fn_ctx: &'a mut FunctionBuilderContext,
		module: &mut JITModule,
		ptr_type: Type,
	) -> Result<Self, AssemblyError> {
		let data_id = module.declare_anonymous_data(true, false)?;

		let tape_ptr = module.declare_data_in_func(data_id, func);

		{
			let mut data = DataDescription::new();

			data.define_zeroinit(30_000);

			module.define_data(data_id, &data)?;
		}

		let mut builder = FunctionBuilder::new(func, fn_ctx);

		let block = builder.create_block();

		builder.switch_to_block(block);
		builder.append_block_params_for_function_params(block);

		let memory_address = builder.ins().global_value(ptr_type, tape_ptr);

		let exit_block = builder.create_block();
		builder.append_block_param(exit_block, ptr_type);

		let write = {
			let mut write_sig = module.make_signature();
			write_sig.params.push(AbiParam::new(types::I8));
			write_sig.returns.push(AbiParam::new(ptr_type));
			module.declare_function("write", Linkage::Import, &write_sig)?
		};

		let read = {
			let mut read_sig = module.make_signature();
			read_sig.params.push(AbiParam::new(ptr_type));
			read_sig.returns.push(AbiParam::new(ptr_type));
			module.declare_function("read", Linkage::Import, &read_sig)?
		};

		let write = module.declare_func_in_func(write, builder.func);
		let read = module.declare_func_in_func(read, builder.func);

		Ok(Self {
			ptr_type,
			builder,
			read,
			write,
			exit_block,
			memory_address,
		})
	}

	fn assemble(mut self, compiler: Compiler) -> Result<(), AssemblyError> {
		self.ops(&compiler)?;

		let Self {
			ptr_type,
			exit_block,
			..
		} = self;

		let zero = self.ins().iconst(ptr_type, 0);

		self.ins().return_(&[zero]);

		self.switch_to_block(exit_block);
		self.seal_block(exit_block);

		let result = self.block_params(exit_block)[0];

		self.set_cold_block(exit_block);
		self.ins().return_(&[result]);

		self.seal_all_blocks();

		self.builder.finalize();

		Ok(())
	}

	fn ops(&mut self, ops: &[BrainMlir]) -> Result<(), AssemblyError> {
		for op in ops {
			match op {
				BrainMlir::ChangeCell(i, offset) => {
					self.change_cell(*i, offset.map_or(0, NonZero::get));
				}
				BrainMlir::MovePointer(offset) => self.move_pointer(*offset),
				BrainMlir::DynamicLoop(ops) => self.dynamic_loop(ops)?,
				BrainMlir::PutOutput => self.put_output(),
				BrainMlir::GetInput => self.get_input(),
				BrainMlir::SetCell(value, offset) => {
					self.set_cell(*value, offset.map_or(0, NonZero::get));
				}
				BrainMlir::ScaleAndMoveValue(factor, offset) => {
					self.scale_and_move_value(*factor, *offset);
				}
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
			}
		}

		Ok(())
	}

	fn load(&mut self, offset: i32) -> Value {
		let memory_address = self.memory_address;
		self.ins()
			.load(types::I8, MemFlags::new(), memory_address, offset)
	}

	fn store(&mut self, value: Value, offset: i32) {
		let memory_address = self.memory_address;
		self.ins()
			.store(MemFlags::new(), value, memory_address, offset);
	}

	fn const_u8(&mut self, value: u8) -> Value {
		self.ins().iconst(types::I8, i64::from(value))
	}

	fn move_pointer(&mut self, offset: i32) {
		let ptr_type = self.ptr_type;
		let memory_address = self.memory_address;

		let value = self.ins().iconst(ptr_type, i64::from(offset));
		self.memory_address = self.ins().iadd(memory_address, value);
	}

	fn scale_and_move_value(&mut self, factor: u8, offset: i32) {
		let current_value = self.load(0);
		self.set_cell(0, 0);

		let other_cell = self.load(offset);

		let value_to_add = self.ins().imul_imm(current_value, i64::from(factor));

		let added = self.ins().iadd(other_cell, value_to_add);

		self.store(added, offset);
	}

	fn change_cell(&mut self, value: i8, offset: i32) {
		let heap_value = self.load(offset);
		let changed = self.ins().iadd_imm(heap_value, i64::from(value));
		self.store(changed, offset);
	}

	fn dynamic_loop(&mut self, ops: &[BrainMlir]) -> Result<(), AssemblyError> {
		let ptr_type = self.ptr_type;
		let memory_address = self.memory_address;

		let head_block = self.create_block();
		let body_block = self.create_block();
		let next_block = self.create_block();

		self.append_block_param(head_block, ptr_type);
		self.append_block_param(body_block, ptr_type);
		self.append_block_param(next_block, ptr_type);

		self.ins().jump(head_block, &[memory_address.into()]);

		self.switch_to_block(head_block);
		self.memory_address = self.block_params(head_block)[0];

		let value = self.load(0);
		let memory_address = self.memory_address;

		self.ins().brif(
			value,
			body_block,
			&[memory_address.into()],
			next_block,
			&[memory_address.into()],
		);

		self.switch_to_block(body_block);
		self.ops(ops)?;

		let memory_address = self.memory_address;
		self.ins().jump(head_block, &[memory_address.into()]);

		self.switch_to_block(next_block);
		self.memory_address = self.block_params(next_block)[0];

		self.set_cell(0, 0);

		Ok(())
	}

	fn set_cell(&mut self, value: u8, offset: i32) {
		let value = self.const_u8(value);
		self.store(value, offset);
	}

	fn put_output(&mut self) {
		let write = self.write;
		let exit_block = self.exit_block;

		let value = self.load(0);
		let inst = self.ins().call(write, &[value]);
		let result = self.inst_results(inst)[0];

		let after_block = self.create_block();

		self.ins()
			.brif(result, exit_block, &[result.into()], after_block, &[]);

		self.switch_to_block(after_block);
		self.seal_block(after_block);
	}

	fn get_input(&mut self) {
		let read = self.read;
		let memory_address = self.memory_address;
		let exit_block = self.exit_block;

		let inst = self.ins().call(read, &[memory_address]);
		let result = self.inst_results(inst)[0];

		let after_block = self.create_block();

		self.ins()
			.brif(result, exit_block, &[result.into()], after_block, &[]);

		self.switch_to_block(after_block);
		self.seal_block(after_block);
	}
}

impl<'a> Deref for Assembler<'a> {
	type Target = FunctionBuilder<'a>;

	fn deref(&self) -> &Self::Target {
		&self.builder
	}
}

impl DerefMut for Assembler<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.builder
	}
}

#[derive(Debug)]
pub enum AssemblyError {
	NoModuleFound,
	Set(settings::SetError),
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
			Self::Set(..) => f.write_str("could not set flag value"),
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
			Self::Set(e) => Some(e),
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
		Self::Set(value)
	}
}

impl From<CodegenError> for AssemblyError {
	fn from(value: CodegenError) -> Self {
		Self::Codegen(value)
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

unsafe extern "C" fn write(value: u8) -> *mut IoError {
	if cfg!(target_os = "windows") && value >= 128 {
		return ptr::null_mut();
	}

	let mut stdout = io::stdout().lock();

	let result = stdout.write_all(&[value]).and_then(|()| stdout.flush());

	match result {
		Ok(()) => ptr::null_mut(),
		Err(e) => Box::into_raw(Box::new(e)),
	}
}

unsafe extern "C" fn read(buf: *mut u8) -> *mut IoError {
	let mut stdin = io::stdin().lock();
	loop {
		let mut value = 0;
		let err = stdin.read_exact(slice::from_mut(&mut value));

		if let Err(e) = err {
			if !matches!(e.kind(), io::ErrorKind::UnexpectedEof) {
				return Box::into_raw(Box::new(e));
			}

			value = 0;
		}

		if cfg!(target_os = "windows") && matches!(value, b'\r') {
			continue;
		}

		unsafe { *buf = value };

		break ptr::null_mut();
	}
}
