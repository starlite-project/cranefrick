use std::{fs, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use koopa::back::KoopaGenerator;
use koopa_test::build_program;

fn main() -> Result<()> {
	let args = match Args::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};

	let raw_source = fs::read_to_string(args.file_path)?;

	// let out_file = args.output_path.join("output.koopa_ir");
	let program = build_program(&raw_source)?;

	let out_file = {
		let path = args.output_path.join("output.koopa_ir");

		fs::OpenOptions::new()
			.create(true)
			.write(true)
			.truncate(true)
			.open(path)?
	};

	KoopaGenerator::new(out_file).generate_on(&program)?;

	Ok(())
}

#[derive(Debug, Parser)]
struct Args {
	pub file_path: PathBuf,
	#[arg(short, long)]
	pub output_path: PathBuf,
}
