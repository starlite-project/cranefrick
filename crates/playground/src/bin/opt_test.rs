use std::fs;

use anyhow::Result;
use cranelift::{
	codegen::{
		cfg_printer::CFGPrinter, control::ControlPlane, ir::{Function, UserFuncName}, Context
	},
	prelude::*,
};
use target_lexicon::Triple;

fn main() -> Result<()> {
	let isa = isa::lookup(Triple::host())?.finish(settings::Flags::new({
		let mut flags_builder = settings::builder();

		flags_builder.enable("enable_pcc")?;
		flags_builder.set("opt_level", "speed_and_size")?;

		flags_builder
	}))?;

	let mut sig = Signature::new(isa.default_call_conv());

	sig.returns.push(AbiParam::new(types::I64));

	let mut fn_ctx = FunctionBuilderContext::new();
	let mut func = Function::with_name_signature(UserFuncName::testcase("sample"), sig);

	{
		let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

		let block = builder.create_block();
		builder.switch_to_block(block);
		builder.seal_block(block);

		let first = builder.ins().iconst(types::I64, 2);
		let second = builder.ins().iconst(types::I64, 3);

		let res = builder.ins().iadd(first, second);

		builder.ins().return_(&[res]);

		builder.finalize();
	};

	let mut ctx = Context::for_function(func);

	println!("Unoptimized");
	println!("{}", ctx.func);

	ctx.optimize(&*isa, &mut ControlPlane::default())?;

	println!("Optimized");
	println!("{}", ctx.func);

	write_dot_graph(&ctx.func)?;

	fs::write(
		"../../out/playground_program.bin",
		ctx.compile(&*isa, &mut ControlPlane::default())
			.unwrap()
			.code_buffer(),
	)?;

	Ok(())
}

fn write_dot_graph(f: &Function) -> Result<()> {
	let writer = CFGPrinter::new(f);

	let mut out = String::new();

	writer.write(&mut out)?;

	fs::write("../../out/program.dot", out)?;

	Ok(())
}
