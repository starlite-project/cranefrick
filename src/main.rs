use std::{
	fs,
	io::{self, Error as IoError, prelude::*},
	mem,
	ops::{Deref, DerefMut},
	path::PathBuf,
	ptr, slice,
};

use clap::Parser;
use color_eyre::Result;
use cranefrick_hlir::{BrainHlir, Parser as BrainParser};
use cranefrick_mlir::{BrainMlir, Compiler};
use cranelift::{
	codegen::{
		Context,
		control::ControlPlane,
		ir::{Function, SigRef, UserFuncName},
		verify_function,
	},
	prelude::*,
};
use ron::ser::PrettyConfig;
use serde::Serialize as _;
use target_lexicon::Triple;
use tracing_error::ErrorLayer;
use tracing_subscriber::{
	EnvFilter,
	fmt::{self, format::FmtSpan},
	prelude::*,
};

fn main() -> Result<()> {
	install_tracing();
	color_eyre::install()?;

	let args = match Args::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};

	let raw_data = fs::read_to_string(args.file_path)?;

	let parsed = BrainParser::new(&raw_data).parse::<Vec<_>>()?;

	let code = generate_ir(parsed)?;

	let mut buffer = memmap2::MmapOptions::new().len(code.len()).map_anon()?;

	buffer.copy_from_slice(&code);

	let buffer = buffer.make_exec()?;

	let mut tape = [0u8; 30_000];

	unsafe {
		let code_fn: unsafe extern "C" fn(*mut u8) -> *mut IoError =
			mem::transmute(buffer.as_ptr());

		let error = code_fn(tape.as_mut_ptr());

		if !error.is_null() {
			let e = Err(*Box::from_raw(error));

			e?;
		}
	}

	Ok(())
}

fn generate_ir(parsed: impl IntoIterator<Item = BrainHlir>) -> Result<Vec<u8>> {
	let current_triple = Triple::host();

	let mut settings_builder = settings::builder();
	settings_builder.set("opt_level", "speed_and_size")?;
	settings_builder.set("stack_switch_model", "update_windows_tib")?;
	settings_builder.enable("enable_pcc")?;

	let isa = isa::lookup(current_triple)?.finish(settings::Flags::new(settings_builder))?;

	let ptr_type = isa.pointer_type();

	let mut sig = Signature::new(isa.default_call_conv());
	sig.params.push(AbiParam::new(ptr_type));
	sig.returns.push(AbiParam::new(ptr_type));

	let mut fn_builder_ctx = FunctionBuilderContext::new();
	let mut func = Function::with_name_signature(UserFuncName::user(0, 0), sig);

	let mut builder = FunctionBuilder::new(&mut func, &mut fn_builder_ctx);

	let ptr = Variable::new(0);
	builder.declare_var(ptr, ptr_type);

	let exit_block = builder.create_block();
	builder.append_block_param(exit_block, ptr_type);

	let block = {
		let block = builder.create_block();
		builder.seal_block(block);
		builder.append_block_params_for_function_params(block);
		builder.switch_to_block(block);
		block
	};

	let zero = builder.ins().iconst(ptr_type, 0);
	let memory_address = builder.block_params(block)[0];

	let write = {
		let mut write_sig = Signature::new(isa.default_call_conv());
		write_sig.params.push(AbiParam::new(types::I8));
		write_sig.returns.push(AbiParam::new(ptr_type));
		let write_sig = builder.import_signature(write_sig);

		let write_addr = write as *const () as i64;
		let write_addr = builder.ins().iconst(ptr_type, write_addr);
		(write_sig, write_addr)
	};

	let read = {
		let mut read_sig = Signature::new(isa.default_call_conv());
		read_sig.params.push(AbiParam::new(ptr_type));
		read_sig.returns.push(AbiParam::new(ptr_type));
		let read_sig = builder.import_signature(read_sig);

		let read_addr = read as *const () as i64;
		let read_addr = builder.ins().iconst(ptr_type, read_addr);
		(read_sig, read_addr)
	};

	let mut compiler = Compiler::from_iter(parsed);

	serialize_compiler(&compiler, "unoptimized")?;

	compiler.optimize();

	serialize_compiler(&compiler, "optimized")?;

	let mut builder = OpsGenerator {
		builder,
		ptr_var: ptr,
		memory_address,
		ptr_type,
		write,
		read,
		exit_block,
		jump_stack: Vec::new(),
	}
	.generate(compiler);

	{
		builder.ins().return_(&[zero]);

		builder.switch_to_block(exit_block);
		builder.seal_block(exit_block);

		let result = builder.block_params(exit_block)[0];
		builder.ins().return_(&[result]);

		builder.seal_all_blocks();

		builder.finalize();
	}

	verify_function(&func, &*isa)?;
	fs::write("./out/unoptimized.clif", func.display().to_string())?;

	let (optimized, code) = {
		let mut ctx = Context::for_function(func);
		let mut plane = ControlPlane::default();

		ctx.optimize(&*isa, &mut plane)?;

		(
			ctx.func.clone(),
			ctx.compile(&*isa, &mut plane).unwrap().clone(),
		)
	};

	let buffer = code.code_buffer().to_owned();

	fs::write("./out/program.bin", &buffer)?;
	fs::write("./out/optimized.clif", optimized.display().to_string())?;

	Ok(buffer)
}

unsafe extern "C" fn write(value: u8) -> *mut IoError {
	if cfg!(target_os = "windows") && value >= 128 {
		return ptr::null_mut();
	}

	let mut stdout = io::stdout().lock();

	let result = stdout.write_all(&[value]).and_then(|_| stdout.flush());

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

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
	pub file_path: PathBuf,
}

struct OpsGenerator<'a> {
	builder: FunctionBuilder<'a>,
	ptr_var: Variable,
	memory_address: Value,
	ptr_type: Type,
	write: (SigRef, Value),
	read: (SigRef, Value),
	exit_block: Block,
	jump_stack: Vec<(Block, Block)>,
}

impl<'a> OpsGenerator<'a> {
	fn generate(mut self, compiler: Compiler) -> FunctionBuilder<'a> {
		self.ops(&compiler);

		self.builder
	}

	fn ops(&mut self, ops: &[BrainMlir]) {
		for op in ops {
			match op {
				BrainMlir::SetCell(v) => self.set_cell(*v),
				BrainMlir::ChangeCell(v) => self.change_cell(*v),
				BrainMlir::MovePtr(v) => self.move_ptr(*v),
				BrainMlir::DynamicLoop(ops) => self.dynamic_loop(ops),
				BrainMlir::GetInput => self.get_input(),
				BrainMlir::PutOutput => self.put_output(),
				BrainMlir::JumpRight => self.jump_right(),
				BrainMlir::JumpLeft => self.jump_left(),
				i => unimplemented!("{i:?}"),
			}
		}
	}

	fn store(&mut self, value: Value) {
		let addr = self.cell_addr();

		self.ins().store(MemFlags::new(), value, addr, 0);
	}

	fn change_cell(&mut self, v: i8) {
		let cell_value = self.cell_value();
		let cell_value = self.ins().iadd_imm(cell_value, i64::from(v));
		self.store(cell_value);
	}

	fn set_cell(&mut self, value: i8) {
		let value = self.ins().iconst(types::I8, i64::from(value));
		self.store(value);
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

		self.builder.def_var(self.ptr_var, ptr_value);
	}

	fn jump_right(&mut self) {
		let inner_block = self.create_block();
		let after_block = self.create_block();

		let cell_value = self.cell_value();

		self.ins()
			.brif(cell_value, inner_block, &[], after_block, &[]);
		self.switch_to_block(inner_block);

		self.jump_stack.push((inner_block, after_block));
	}

	fn jump_left(&mut self) {
		let (inner_block, after_block) = self.jump_stack.pop().unwrap();
		let cell_value = self.cell_value();

		self.ins()
			.brif(cell_value, inner_block, &[], after_block, &[]);

		self.seal_block(inner_block);
		self.seal_block(after_block);

		self.switch_to_block(after_block);
	}

	fn dynamic_loop(&mut self, ops: &[BrainMlir]) {
		let head = self.create_block();

		let body = self.create_block();

		let next = self.create_block();

		self.ins().jump(head, &[]);

		self.switch_to_block(head);

		let cell_value = self.cell_value();

		self.ins().brif(cell_value, body, &[], next, &[]);

		self.switch_to_block(body);
		self.ops(ops);
		self.ins().jump(head, &[]);

		self.switch_to_block(next);
		self.set_cell(0);
	}

	fn ptr_value(&mut self) -> Value {
		self.builder.use_var(self.ptr_var)
	}

	fn put_output(&mut self) {
		let (write_sig, write_addr) = self.write;
		let exit_block = self.exit_block;

		let cell_value = self.cell_value();

		let inst = self
			.ins()
			.call_indirect(write_sig, write_addr, &[cell_value]);
		let result = self.inst_results(inst)[0];

		let after_block = self.create_block();

		self.ins()
			.brif(result, exit_block, &[result.into()], after_block, &[]);

		self.seal_block(after_block);
		self.switch_to_block(after_block);
	}

	fn get_input(&mut self) {
		let exit_block = self.exit_block;
		let (read_sig, read_addr) = self.read;
		let cell_addr = self.cell_addr();

		let inst = self.ins().call_indirect(read_sig, read_addr, &[cell_addr]);
		let result = self.inst_results(inst)[0];

		let after_block = self.create_block();

		self.ins()
			.brif(result, exit_block, &[result.into()], after_block, &[]);
		self.seal_block(after_block);
		self.switch_to_block(after_block);
	}

	fn cell_addr(&mut self) -> Value {
		let ptr_value = self.ptr_value();
		let addr = self.memory_address;
		self.ins().iadd(addr, ptr_value)
	}

	fn cell_value(&mut self) -> Value {
		let addr = self.cell_addr();
		self.ins().load(types::I8, MemFlags::new(), addr, 0)
	}
}

impl<'a> Deref for OpsGenerator<'a> {
	type Target = FunctionBuilder<'a>;

	fn deref(&self) -> &Self::Target {
		&self.builder
	}
}

impl<'a> DerefMut for OpsGenerator<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.builder
	}
}

fn install_tracing() {
	fs::create_dir_all("./out").unwrap();

	let log_file = fs::OpenOptions::new()
		.create(true)
		.write(true)
		.truncate(true)
		.open("./out/output.log")
		.expect("failed to create log file");

	let json_log_file = fs::OpenOptions::new()
		.create(true)
		.truncate(true)
		.write(true)
		.open("./out/output.json")
		.expect("failed to create json log file");

	let file_layer = fmt::layer().with_ansi(false).with_writer(log_file);

	let filter_layer = EnvFilter::new("info");
	let fmt_layer = fmt::layer().with_target(false).with_filter(filter_layer);

	let json_file_layer = fmt::layer()
		.with_ansi(false)
		.json()
		.flatten_event(true)
		.with_span_events(FmtSpan::FULL)
		.with_writer(json_log_file);

	tracing_subscriber::registry()
		.with(json_file_layer)
		.with(file_layer)
		.with(fmt_layer)
		.with(ErrorLayer::default())
		.init();
}

fn serialize_compiler(comp: &Compiler, file_name: &str) -> Result<()> {
	let mut output = String::new();
	let mut serializer = ron::Serializer::with_options(
		&mut output,
		Some(PrettyConfig::new().separate_tuple_members(true)),
		&ron::Options::default(),
	)?;

	comp.serialize(&mut serializer)?;

	fs::write(format!("./out/{file_name}.ron"), output)?;

	Ok(())
}
