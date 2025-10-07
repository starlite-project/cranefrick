mod args;

use clap::Parser as _;
use color_eyre::{Report, Result};
use inkwell::{
	context::Context,
	values::{BasicValue, CallSiteValue},
};

use self::args::Args;

fn main() -> Result<()> {
	color_eyre::install()?;

	let args = match Args::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};

	let context = Context::create();
	let input = inkwell::module::Module::parse_bitcode_from_path(&args.file_path, &context)
		.map_err(|s| Report::msg(s.to_string()))?;

	for function in input.get_functions() {
		for block in function.get_basic_block_iter() {
			for instr in block.get_instructions() {
				let Ok(call_site) = CallSiteValue::try_from(instr) else {
					continue;
				};

				let Some(fn_value) = call_site.get_called_fn_value() else {
					continue;
				};

				let name = fn_value.get_name().to_string_lossy();

				if !name.contains("memcpy") {
					continue;
				}

				let Some(size_param) = fn_value.get_nth_param(2) else {
					continue;
				};
			}
		}
	}

	Ok(())
}
