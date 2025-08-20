mod compiler;

use std::{fs, path::PathBuf};

use clap::Parser;
use color_eyre::{Report, Result, eyre::ContextCompat};
use inkwell::{
	OptimizationLevel,
	context::Context,
	passes::PassBuilderOptions,
	targets::{CodeModel, RelocMode, Target, TargetMachine},
};

use self::compiler::Compiler;

const PASSES: &str = "default<O3>,aa-eval,instcount,lint,adce,break-crit-edges,dse,instcombine,internalize,jump-threading,lcssa,loop-deletion,loop-rotate,loop-simplify,loop-unroll,mem2reg,memcpyopt,reassociate,simplifycfg,sink,simple-loop-unswitch,strip,tailcallelim,transform-warning";

fn main() -> Result<()> {
	color_eyre::install()?;

	let args = match Args::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};

	let raw_source = fs::read_to_string(&args.file_path)?;

	Compiler::init_targets();

	let context = Context::create();
	let module = context.create_module("cranefrick-rust-llvm");

	let compiler = Compiler {
		context: &context,
		module,
		builder: context.create_builder(),
	};

	compiler.compile(&raw_source)?;

	let Compiler { module, .. } = compiler;

	{
		fs::write(
			"../../out/unoptimized.ir",
			module.print_to_string().to_string(),
		)?;
	}

	let target_triple = TargetMachine::get_default_triple();
	let cpu = TargetMachine::get_host_cpu_name().to_string();
	let features = TargetMachine::get_host_cpu_features().to_string();

	let target = Target::from_triple(&target_triple).map_err(|e| Report::msg(e.to_string()))?;

	let target_machine = target
		.create_target_machine(
			&target_triple,
			&cpu,
			&features,
			OptimizationLevel::Aggressive,
			RelocMode::Default,
			CodeModel::Default,
		)
		.context("Unable to create target machine")?;

	let pass_options = PassBuilderOptions::create();

	pass_options.set_verify_each(true);
	pass_options.set_debug_logging(true);
	pass_options.set_call_graph_profile(true);

	module
		.run_passes(PASSES, &target_machine, pass_options)
		.map_err(|e| Report::msg(e.to_string()))?;

	{
		fs::write(
			"../../out/optimized.ir",
			module.print_to_string().to_string(),
		)?;
	}

	Ok(())
}

#[derive(Debug, Parser)]
struct Args {
	file_path: PathBuf,
}
