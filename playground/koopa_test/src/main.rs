use std::{fs, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use koopa::{
	back::KoopaGenerator,
	opt::{Pass, PassManager},
};
use koopa_test::{ConstantFolding, build_program};

fn main() -> Result<()> {
	let args = match Args::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};

	let raw_source = fs::read_to_string(args.file_path)?;

	let mut program = build_program(&raw_source)?;

	let unoptimized_out_file = {
		let path = args.output_path.join("unoptimized.koopa_ir");

		fs::OpenOptions::new()
			.create(true)
			.write(true)
			.truncate(true)
			.open(path)?
	};

	KoopaGenerator::new(unoptimized_out_file).generate_on(&program)?;

	let mut pass_manager = PassManager::new();

	pass_manager.register(Pass::Function(Box::new(ConstantFolding::new())));

	pass_manager.run_passes(&mut program);

	let optimized_out_file = {
		let path = args.output_path.join("optimized.koopa_ir");

		fs::OpenOptions::new()
			.create(true)
			.write(true)
			.truncate(true)
			.open(path)?
	};

	KoopaGenerator::new(optimized_out_file).generate_on(&program)?;

	Ok(())
}

#[derive(Debug, Parser)]
struct Args {
	pub file_path: PathBuf,
	#[arg(short, long)]
	pub output_path: PathBuf,
}
