#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

use std::{
	error::Error as StdError,
	fmt::{Debug, Display, Formatter, Result as FmtResult},
	fs,
	io::{self, Error as IoError, prelude::*},
	ops::{Deref, DerefMut},
	path::Path,
	ptr, slice,
};

use cranefrick_mlir::{BrainMlir, Compiler};
use cranelift::{
	codegen::{CodegenError, control::ControlPlane, ir::FuncRef},
	jit::{JITBuilder, JITModule},
	module::{FuncId, Linkage, Module, ModuleError},
	prelude::*,
};

pub struct AssembledModule {
	module: Option<JITModule>,
	main: FuncId,
}

impl AssembledModule {
	pub fn assemble(compiler: Compiler, output_path: &Path) -> Result<Self, AssemblyError> {
		let mut flag_builder = settings::builder();

		flag_builder.set("opt_level", "speed_and_size")?;
		flag_builder.set("stack_switch_model", "update_windows_tib")?;
		flag_builder.enable("enable_pcc")?;

		let isa = cranelift::native::builder()
			.map_err(AssemblyError::Custom)?
			.finish(settings::Flags::new(flag_builder))?;

		let mut jit_builder =
			JITBuilder::with_isa(isa.clone(), cranelift::module::default_libcall_names());

		jit_builder.symbol("write", write as *const u8);
		jit_builder.symbol("read", read as *const u8);

		let mut module = JITModule::new(jit_builder);
		let ptr_type = module.target_config().pointer_type();

		let mut ctx = module.make_context();
		let mut func_ctx = FunctionBuilderContext::new();

		let mut sig = module.make_signature();

		sig.params.extend([AbiParam::new(ptr_type); 1]);
		sig.returns.push(AbiParam::new(ptr_type));

		let func = module.declare_function("main", Linkage::Local, &sig)?;

		ctx.func.signature = sig;

		{
			let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

			let ptr = Variable::new(0);
			builder.declare_var(ptr, ptr_type);

			let block = builder.create_block();

			builder.switch_to_block(block);
			builder.append_block_params_for_function_params(block);

			let memory_address = builder.block_params(block)[0];

			let exit_block = builder.create_block();
			builder.append_block_param(exit_block, ptr_type);

			let write = {
				let mut write_sig = module.make_signature();
				write_sig.params.push(AbiParam::new(types::I8));
				write_sig.returns.push(AbiParam::new(ptr_type));
				// let write_sig = builder.import_signature(write_sig);

				// let write_addr = write as *const () as i64;
				// let write_addr = builder.ins().iconst(ptr_type, write_addr);
				// (write_sig, write_addr)

				module.declare_function("write", Linkage::Import, &write_sig)?
			};

			let read = {
				let mut read_sig = module.make_signature();
				read_sig.params.push(AbiParam::new(ptr_type));
				read_sig.returns.push(AbiParam::new(ptr_type));
				// let read_sig = builder.import_signature(read_sig);

				// let read_addr = read as *const () as i64;
				// let read_addr = builder.ins().iconst(ptr_type, read_addr);
				// (read_sig, read_addr)
				module.declare_function("read", Linkage::Import, &read_sig)?
			};

			let read = module.declare_func_in_func(read, builder.func);
			let write = module.declare_func_in_func(write, builder.func);

			let assembler = Assembler {
				ptr,
				ptr_type,
				builder,
				read,
				write,
				exit_block,
				memory_address,
			};

			let mut builder = assembler.assemble(compiler)?;

			{
				let zero = builder.ins().iconst(ptr_type, 0);

				builder.ins().return_(&[zero]);

				builder.switch_to_block(exit_block);
				builder.seal_block(exit_block);

				let result = builder.block_params(exit_block)[0];
				builder.ins().return_(&[result]);

				builder.seal_all_blocks();

				builder.finalize();
			}
		}

		fs::write(output_path.join("unoptimized.clif"), ctx.func.to_string())?;

		ctx.verify(&*isa).unwrap();
		ctx.optimize(&*isa, &mut ControlPlane::default())?;
		ctx.verify(&*isa).unwrap();

		fs::write(output_path.join("optimized.clif"), ctx.func.to_string())?;

		module.define_function(func, &mut ctx)?;
		module.clear_context(&mut ctx);

		module.finalize_definitions()?;

		Ok(Self {
			module: Some(module),
			main: func,
		})
	}

	pub fn execute(self) -> Result<(), AssemblyError> {
		let module = self.module.as_ref().ok_or(AssemblyError::NoModuleFound)?;

		let code = module.get_finalized_function(self.main);

		let exec = unsafe { std::mem::transmute::<*const u8, for<'a, 'b> fn(*mut u8)>(code) };

		let mut heap = [0u8; 30_000];

		exec(heap.as_mut_ptr());

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
	ptr: Variable,
	ptr_type: Type,
	builder: FunctionBuilder<'a>,
	read: FuncRef,
	write: FuncRef,
	exit_block: Block,
	memory_address: Value,
}

impl<'a> Assembler<'a> {
	fn assemble(mut self, compiler: Compiler) -> Result<FunctionBuilder<'a>, AssemblyError> {
		self.ops(&compiler)?;

		Ok(self.builder)
	}

	fn change_cell(&mut self, v: i8) {
		let cell_value = self.current_cell_value();
		let cell_value = self.ins().iadd_imm(cell_value, i64::from(v));
		self.store(cell_value);
	}

	fn move_ptr(&mut self, offset: i64) {
		let ptr_type = self.ptr_type;
		let value = self.ptr_value();
		let offset_ptr = self.ins().iadd_imm(value, offset);

		let ptr_value = if offset < 0 {
			let wrapped = self.ins().iconst(ptr_type, 30_000 - offset);
			self.ins().select(value, offset_ptr, wrapped)
		} else {
			let cmp = self.ins().icmp_imm(IntCC::Equal, offset_ptr, 30_000);
			let zero = self.ins().iconst(ptr_type, 0);

			self.ins().select(cmp, zero, offset_ptr)
		};

		let ptr = self.ptr;

		self.def_var(ptr, ptr_value);
	}

	fn get_input(&mut self) {
		let exit_block = self.exit_block;
		let read = self.read;
		let cell_addr = self.memory_address();

		let inst = self.ins().call(read, &[cell_addr]);

		let result = self.inst_results(inst)[0];

		let after_block = self.create_block();

		self.ins()
			.brif(result, exit_block, &[result.into()], after_block, &[]);
		self.seal_block(after_block);
		self.switch_to_block(after_block);
	}

	fn dynamic_loop(&mut self, ops: &[BrainMlir]) -> Result<(), AssemblyError> {
		let head_block = self.create_block();
		let body_block = self.create_block();
		let next_block = self.create_block();

		self.ins().jump(head_block, &[]);

		self.switch_to_block(head_block);

		let cell_value = self.current_cell_value();

		self.ins()
			.brif(cell_value, body_block, &[], next_block, &[]);

		self.switch_to_block(body_block);
		self.ops(ops)?;
		self.ins().jump(head_block, &[]);

		self.switch_to_block(next_block);
		self.set_cell(0);

		Ok(())
	}

	fn set_cell(&mut self, v: u8) {
		let value = self.ins().iconst(types::I8, i64::from(v));
		self.store(value);
	}

	fn put_output(&mut self) {
		let write = self.write;
		let exit_block = self.exit_block;
		let cell_value = self.current_cell_value();

		let inst = self.ins().call(write, &[cell_value]);
		let result = self.inst_results(inst)[0];

		let after_block = self.create_block();

		self.ins()
			.brif(result, exit_block, &[result.into()], after_block, &[]);

		self.seal_block(after_block);
		self.switch_to_block(after_block);
	}

	fn ops(&mut self, ops: &[BrainMlir]) -> Result<(), AssemblyError> {
		for op in ops {
			match op {
				BrainMlir::ChangeCell(i) => self.change_cell(*i),
				BrainMlir::MovePtr(offset) => self.move_ptr(*offset),
				BrainMlir::DynamicLoop(l) => self.dynamic_loop(l)?,
				BrainMlir::SetCell(v) => self.set_cell(*v),
				BrainMlir::PutOutput => self.put_output(),
				BrainMlir::GetInput => self.get_input(),
				i => return Err(AssemblyError::NotImplemented(i.clone())),
			}
		}

		Ok(())
	}

	fn store(&mut self, value: Value) {
		let addr = self.memory_address();
		self.ins().store(MemFlags::new(), value, addr, 0);
	}

	fn ptr_value(&mut self) -> Value {
		let ptr = self.ptr;
		self.use_var(ptr)
	}

	fn memory_address(&mut self) -> Value {
		let ptr_value = self.ptr_value();
		let addr = self.memory_address;
		self.ins().iadd(addr, ptr_value)
	}

	fn current_cell_value(&mut self) -> Value {
		let addr = self.memory_address();
		self.ins().load(types::I8, MemFlags::new(), addr, 0)
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

unsafe extern "C" fn write(value: u8) -> *mut IoError {
	if cfg!(target_os = "windows") && value >= 128 {
		return ptr::null_mut();
	}

	let mut stdout = io::stdout().lock();

	let result = stdout.write_all(&[value]).and_then(|()| stdout.flush());

	match result {
		Err(e) => Box::into_raw(Box::new(e)),
		_ => ptr::null_mut(),
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

		return ptr::null_mut();
	}
}
