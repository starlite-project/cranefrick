use std::fs;

use anyhow::Result;
use cranelift::{
	codegen::{
		Context,
		cfg_printer::CFGPrinter,
		control::ControlPlane,
		ir::{Function, UserFuncName},
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

	sig.params.push(AbiParam::new(types::I8X2));
	sig.returns.push(AbiParam::new(types::I8X2));

	let mut fn_ctx = FunctionBuilderContext::new();
	let mut func = Function::with_name_signature(UserFuncName::testcase("sample"), sig);

	{
		let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

		let block = builder.create_block();
		builder.append_block_params_for_function_params(block);

		builder.switch_to_block(block);
		builder.seal_block(block);

		let first_param = builder.block_params(block)[0];

		let other = builder.ins().iconst(types::I8X2, 1);

		let added = builder.ins().iadd(first_param, other);

		builder.ins().return_(&[added]);

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
