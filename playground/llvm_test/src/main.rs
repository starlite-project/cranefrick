mod compiler;

use std::{fs, path::PathBuf};

use clap::Parser;
use color_eyre::{eyre::{ContextCompat, Error}, Report, Result};
use inkwell::{
	context::Context, module::Module, passes::{PassBuilderOptions, PassManager}, targets::{CodeModel, RelocMode, Target, TargetMachine}, values::FunctionValue, OptimizationLevel
};

use self::compiler::Compiler;

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

	let target_machine = target.create_target_machine(
		&target_triple,
		&cpu,
		&features,
		OptimizationLevel::Aggressive,
		RelocMode::Default,
		CodeModel::Default,
	).context("Unable to create target machine")?;

	let pass_options = PassBuilderOptions::create();

	module.run_passes("default<O3>", &target_machine, pass_options).map_err(|e| Report::msg(e.to_string()))?;

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
