use std::{
	fs,
	io::{self, prelude::*},
	path::PathBuf,
};

use clap::Parser;
use color_eyre::Result;
use cranium_isle::{codegen::CodegenOptions, compile, error::Errors};

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

	let code = compile::from_files(args.inputs, &CodegenOptions::default()).unwrap();

	let stdout = io::stdout().lock();

	let (mut output, output_name): (Box<dyn Write>, _) = if let Some(f) = &args.output {
		let output = Box::new(fs::File::create(f)?);
		(output, f.display().to_string())
	} else {
		let output = Box::new(stdout);
		(output, "<stdout>".to_owned())
	};

	output.write_all(code.as_bytes())?;

	Ok(())
}

fn install_tracing() {
	tracing_subscriber::fmt()
		.with_target(false)
		.compact()
		.init();
}

#[derive(Debug, Parser)]
struct Args {
	#[arg(short, long)]
	output: Option<PathBuf>,
	#[arg(required = true)]
	inputs: Vec<PathBuf>,
}
