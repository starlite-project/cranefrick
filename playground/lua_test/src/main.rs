use std::{fs::File, io::Read};

use clap::{Arg, Command, crate_description, crate_name, crate_version};
use color_eyre::{Result, eyre::ContextCompat};
use piccolo::{
	compiler::{self, CompiledPrototype, interning::BasicInterner, string_utils::debug_utf8_lossy},
	io,
};

fn main() -> Result<()> {
	color_eyre::install()?;

	let matches = Command::new(crate_name!())
		.version(crate_version!())
		.about(crate_description!())
		.arg(
			Arg::new("parse")
				.short('p')
				.long("parse")
				.help("Parse file only and output AST"),
		)
		.arg(
			Arg::new("file")
				.required(true)
				.help("File to compile")
				.index(1),
		)
		.get_matches();

	let mut file = io::buffered_read(File::open(
		matches
			.get_one::<String>("file")
			.context("no file provided")?,
	)?)?;

	let mut source = Vec::new();
	file.read_to_end(&mut source)?;

	let mut interner = BasicInterner::default();

	let chunk = compiler::parse_chunk(&source, &mut interner)?;
	if matches.contains_id("parse") {
		println!("{chunk:#?}");
	} else {
		let prototype = compiler::compile_chunk(&chunk, &mut interner)?;
		print_function(&prototype, 0);
	}

	Ok(())
}

fn print_function<S>(function: &CompiledPrototype<S>, depth: usize)
where
	S: AsRef<[u8]>,
{
	let indent = "  ".repeat(depth);
	println!("{indent}===FunctionProto({function:p})===");
	println!(
		"{indent}fixed_params: {}, has_varargs: {}, stack_size: {}",
		function.fixed_params, function.has_varargs, function.stack_size
	);

	if !function.constants.is_empty() {
		println!("{indent}---constants---");
		for (i, c) in function.constants.iter().enumerate() {
			println!(
				"{indent}{i}: {:?}",
				c.as_string_ref()
					.map_string(|s| debug_utf8_lossy(s.as_ref()))
			);
		}
	}

	if !function.opcodes.is_empty() {
		println!("{indent}---opcodes---");

		let mut line_number_ind = 0;
		println!("{indent}<line {}>", function.opcode_line_numbers[0].1);

		for (i, c) in function.opcodes.iter().enumerate() {
			if let Some(&(opcode_index, line_number)) =
				function.opcode_line_numbers.get(line_number_ind + 1)
				&& i >= opcode_index
			{
				line_number_ind += 1;
				println!("{indent}<line {line_number}>");
			}

			println!("{indent}{i}: {c:?}");
		}
	}

	if !function.upvalues.is_empty() {
		println!("{indent}---upvalues---");
		for (i, u) in function.upvalues.iter().enumerate() {
			println!("{indent}{i}: {u:?}");
		}
	}

	if !function.prototypes.is_empty() {
		println!("{indent}---prototypes---");
		for p in &function.prototypes {
			print_function(p, depth + 1);
		}
	}
}
