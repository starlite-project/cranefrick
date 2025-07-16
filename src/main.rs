use std::{
	fs,
	io::{self, Error as IoError, prelude::*},
	mem,
	path::PathBuf,
	ptr, slice,
};

use anyhow::{Context as _, Result};
use clap::Parser;
use cranefrick_hir::{BrainHir, Parser as BrainParser};
use cranelift::{
	codegen::{
		Context,
		control::ControlPlane,
		ir::{Function, UserFuncName},
		verify_function,
	},
	prelude::*,
};
use target_lexicon::Triple;

fn main() -> Result<()> {
	let args = match Args::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};

	_ = fs::remove_dir_all("./out");

	fs::create_dir_all("./out")?;

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

fn generate_ir(parsed: impl IntoIterator<Item = BrainHir>) -> Result<Vec<u8>> {
	let current_triple = Triple::host();

	let mut settings_builder = settings::builder();
	settings_builder.set("opt_level", "speed_and_size")?;

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

	let mut stack = Vec::<(Block, Block)>::new();
	let mem_flags = MemFlags::new();

	let (write_sig, write_addr) = {
		let mut write_sig = Signature::new(isa.default_call_conv());
		write_sig.params.push(AbiParam::new(types::I8));
		write_sig.returns.push(AbiParam::new(ptr_type));
		let write_sig = builder.import_signature(write_sig);

		let write_addr = write as *const () as i64;
		let write_addr = builder.ins().iconst(ptr_type, write_addr);
		(write_sig, write_addr)
	};

	let (read_sig, read_addr) = {
		let mut read_sig = Signature::new(isa.default_call_conv());
		read_sig.params.push(AbiParam::new(ptr_type));
		read_sig.returns.push(AbiParam::new(ptr_type));
		let read_sig = builder.import_signature(read_sig);

		let read_addr = read as *const () as i64;
		let read_addr = builder.ins().iconst(ptr_type, read_addr);
		(read_sig, read_addr)
	};

	for op in parsed {
		match op {
			BrainHir::IncrementCell => {
				let ptr_value = builder.use_var(ptr);
				let cell_addr = builder.ins().iadd(memory_address, ptr_value);
				let cell_value = builder.ins().load(types::I8, mem_flags, cell_addr, 0);
				let cell_value = builder.ins().iadd_imm(cell_value, 1);
				builder.ins().store(mem_flags, cell_value, cell_addr, 0);
			}
			BrainHir::DecrementCell => {
				let ptr_value = builder.use_var(ptr);
				let cell_addr = builder.ins().iadd(memory_address, ptr_value);
				let cell_value = builder.ins().load(types::I8, mem_flags, cell_addr, 0);
				let cell_value = builder.ins().iadd_imm(cell_value, -1);
				builder.ins().store(mem_flags, cell_value, cell_addr, 0);
			}
			BrainHir::MovePtrLeft => {
				let ptr_value = builder.use_var(ptr);
				let ptr_minus_one = builder.ins().iadd_imm(ptr_value, -1);

				let wrapped = builder.ins().iconst(ptr_type, 30_000 - 1);
				let ptr_value = builder.ins().select(ptr_value, ptr_minus_one, wrapped);

				builder.def_var(ptr, ptr_value);
			}
			BrainHir::MovePtrRight => {
				let ptr_value = builder.use_var(ptr);
				let ptr_plus_one = builder.ins().iadd_imm(ptr_value, 1);

				let cmp = builder.ins().icmp_imm(IntCC::Equal, ptr_plus_one, 30_000);
				let ptr_value = builder.ins().select(cmp, zero, ptr_plus_one);

				builder.def_var(ptr, ptr_value);
			}
			BrainHir::StartLoop => {
				let inner_block = builder.create_block();
				let after_block = builder.create_block();

				let ptr_value = builder.use_var(ptr);
				let cell_addr = builder.ins().iadd(memory_address, ptr_value);
				let cell_value = builder.ins().load(types::I8, mem_flags, cell_addr, 0);

				builder
					.ins()
					.brif(cell_value, inner_block, &[], after_block, &[]);

				builder.switch_to_block(inner_block);

				stack.push((inner_block, after_block));
			}
			BrainHir::EndLoop => {
				let (inner_block, after_block) = stack.pop().context("unmatched brackets")?;
				let ptr_value = builder.use_var(ptr);
				let cell_addr = builder.ins().iadd(memory_address, ptr_value);
				let cell_value = builder.ins().load(types::I8, mem_flags, cell_addr, 0);

				builder
					.ins()
					.brif(cell_value, inner_block, &[], after_block, &[]);

				builder.seal_block(inner_block);
				builder.seal_block(after_block);

				builder.switch_to_block(after_block);
			}
			BrainHir::PutOutput => {
				let ptr_value = builder.use_var(ptr);
				let cell_addr = builder.ins().iadd(memory_address, ptr_value);
				let cell_value = builder.ins().load(types::I8, mem_flags, cell_addr, 0);

				let inst = builder
					.ins()
					.call_indirect(write_sig, write_addr, &[cell_value]);
				let result = builder.inst_results(inst)[0];

				let after_block = builder.create_block();

				builder
					.ins()
					.brif(result, exit_block, &[result.into()], after_block, &[]);

				builder.seal_block(after_block);
				builder.switch_to_block(after_block);
			}
			BrainHir::GetInput => {
				let ptr_value = builder.use_var(ptr);
				let cell_addr = builder.ins().iadd(memory_address, ptr_value);

				let inst = builder
					.ins()
					.call_indirect(read_sig, read_addr, &[cell_addr]);
				let result = builder.inst_results(inst)[0];

				let after_block = builder.create_block();

				builder
					.ins()
					.brif(result, exit_block, &[result.into()], after_block, &[]);

				builder.seal_block(after_block);
				builder.switch_to_block(after_block);
			}
		}
	}

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
	fs::write("./out/unoptimized.ir", func.display().to_string())?;

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
	fs::write("./out/optimized.ir", optimized.display().to_string())?;

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
