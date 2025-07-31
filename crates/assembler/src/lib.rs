#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod flags;
mod internal;

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
use cranefrick_utils::PointerExt;
use cranelift_codegen::{
	CodegenError, CompileError,
	cfg_printer::CFGPrinter,
	control::ControlPlane,
	ir::{
		AbiParam, Block, Fact, FuncRef, Function, Inst, InstBuilder as _, MemFlags, Type, Value,
		types,
	},
	isa, settings,
};
use cranelift_frontend::{FuncInstBuilder, FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, FuncId, Linkage, Module as _, ModuleError};
use target_lexicon::Triple;
use tracing::{Span, info, info_span};
use tracing_indicatif::{span_ext::IndicatifSpanExt as _, style::ProgressStyle};

pub use self::flags::*;
use self::internal::FunctionStencilExt as _;

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

		sig.returns.push(AbiParam::new(ptr_type));

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

		let exec = unsafe { std::mem::transmute::<*const u8, fn() -> *mut IoError>(code) };

		let ptr = exec();

		if let Some(error) = unsafe { <*mut IoError as PointerExt<IoError>>::into_boxed(ptr) } {
			Err((*error).into())
		} else {
			Ok(())
		}
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
	memory_address: Value,
	exit_block: Block,
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
			data.set_used(true);

			module.define_data(data_id, &data)?;
		}

		let mut builder = FunctionBuilder::new(func, fn_ctx);

		let block = builder.create_block();

		builder.switch_to_block(block);
		builder.append_block_params_for_function_params(block);

		let memory_address = builder.ins().global_value(ptr_type, tape_ptr);

		{
			let memory_type = builder
				.func
				.create_memory_type(cranelift_codegen::ir::MemoryTypeData::Memory { size: 30_000 });

			let memory_type_fact = Fact::Mem {
				ty: memory_type,
				min_offset: 0,
				max_offset: 30_000,
				nullable: false,
			};

			builder.func.global_value_facts[tape_ptr] = Some(memory_type_fact);
		}

		let exit_block = builder.create_block();
		builder.append_block_param(exit_block, ptr_type);

		builder.set_cold_block(exit_block);

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
			memory_address,
			exit_block,
		})
	}

	#[tracing::instrument("creating cranelift ir", skip_all)]
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

		let result = self.block_params(exit_block)[0];
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
				BrainMlir::OutputCurrentCell => self.output_current_cell(),
				BrainMlir::OutputChar(c) => self.output_char(*c),
				BrainMlir::InputIntoCell => self.input_into_cell(),
				BrainMlir::SetCell(value, offset) => {
					self.set_cell(*value, offset.map_or(0, NonZero::get));
				}
				BrainMlir::MoveValue(factor, offset) => self.move_value(*factor, *offset),
				BrainMlir::TakeValue(factor, offset) => self.take_value(*factor, *offset),
				BrainMlir::FetchValue(factor, offset) => self.fetch_value(*factor, *offset),
				BrainMlir::IfNz(ops) => self.if_nz(ops)?,
				BrainMlir::FindZero(offset) => self.find_zero(*offset),
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
			}
		}

		Ok(())
	}

	#[inline]
	fn ins<'short>(&'short mut self) -> FuncInstBuilder<'short, 'a> {
		self.builder.ins()
	}

	fn load(&mut self, offset: i32) -> Value {
		let memory_address = self.memory_address;
		let value = self
			.ins()
			.load(types::I8, Self::memflags(), memory_address, offset);

		self.func.dfg.facts[value] = Some(Fact::Range {
			bit_width: types::I8.bits() as u16,
			min: 0,
			max: u64::from(u8::MAX),
		});

		value
	}

	fn store(&mut self, value: Value, offset: i32) {
		let memory_address = self.memory_address;
		if self.func.dfg.facts.get(value).is_none() {
			self.func.dfg.facts[value] = Some(Fact::Range {
				bit_width: types::I8.bits() as u16,
				min: 0,
				max: u8::MAX.into(),
			});
		}

		self.ins()
			.store(Self::memflags(), value, memory_address, offset);
	}

	const fn memflags() -> MemFlags {
		MemFlags::trusted()
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

	fn move_value(&mut self, factor: u8, offset: i32) {
		let current_value = self.load(0);
		self.set_cell(0, 0);

		let other_cell = self.load(offset);

		let value_to_add = self.ins().imul_imm(current_value, i64::from(factor));

		let added = self.ins().iadd(other_cell, value_to_add);

		self.store(added, offset);
	}

	fn take_value(&mut self, factor: u8, offset: i32) {
		let current_value = self.load(0);
		self.set_cell(0, 0);

		self.move_pointer(offset);

		let other_cell = self.load(0);

		let value_to_add = self.ins().imul_imm(current_value, i64::from(factor));

		let added = self.ins().iadd(other_cell, value_to_add);

		self.store(added, 0);
	}

	fn fetch_value(&mut self, factor: u8, offset: i32) {
		let other_cell = self.load(offset);
		self.set_cell(0, offset);

		let current_cell = self.load(0);

		let value_to_add = self.ins().imul_imm(other_cell, i64::from(factor));

		let added = self.ins().iadd(current_cell, value_to_add);

		self.store(added, 0);
	}

	fn change_cell(&mut self, value: i8, offset: i32) {
		let heap_value = self.load(offset);
		let changed = if value.is_negative() {
			let sub_value = self.ins().iconst(types::I8, i64::from(value.abs()));
			self.ins().isub(heap_value, sub_value)
		} else {
			self.ins().iadd_imm(heap_value, i64::from(value))
		};

		self.store(changed, offset);
	}

	fn if_nz(&mut self, ops: &[BrainMlir]) -> Result<(), AssemblyError> {
		let ptr_type = self.ptr_type;
		let memory_address = self.memory_address;

		let body_block = self.create_block();
		let next_block = self.create_block();

		self.append_block_param(body_block, ptr_type);
		self.append_block_param(next_block, ptr_type);

		let value = self.load(0);

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
		self.ins().jump(next_block, &[memory_address.into()]);

		self.switch_to_block(next_block);
		self.seal_block(next_block);
		self.memory_address = self.block_params(next_block)[0];

		self.set_cell(0, 0);

		Ok(())
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

	fn find_zero(&mut self, offset: i32) {
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
		let memory_address = self.block_params(head_block)[0];
		self.memory_address = memory_address;

		let value = self.load(0);

		self.ins().brif(
			value,
			body_block,
			&[memory_address.into()],
			next_block,
			&[memory_address.into()],
		);

		self.switch_to_block(body_block);
		self.memory_address = self.block_params(body_block)[0];

		self.move_pointer(offset);
		let memory_address = self.memory_address;

		self.ins().jump(head_block, &[memory_address.into()]);

		self.switch_to_block(next_block);
		self.memory_address = self.block_params(next_block)[0];
	}

	fn set_cell(&mut self, value: u8, offset: i32) {
		let value = self.const_u8(value);
		self.store(value, offset);
	}

	fn output_char(&mut self, c: u8) {
		let write = self.write;

		let value = self.ins().iconst(types::I8, i64::from(c));
		let inst = self.ins().call(write, &[value]);

		self.handle_extern_call(inst);
	}

	fn output_current_cell(&mut self) {
		let write = self.write;

		let value = self.load(0);
		let inst = self.ins().call(write, &[value]);

		self.handle_extern_call(inst);
	}

	fn input_into_cell(&mut self) {
		let read = self.read;
		let memory_address = self.memory_address;

		let inst = self.ins().call(read, &[memory_address]);

		self.handle_extern_call(inst);
	}

	fn handle_extern_call(&mut self, inst: Inst) {
		let exit_block = self.exit_block;

		let result = self.first_result(inst);

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
