mod args;

use std::{fs, path::Path};

use clap::Parser as _;
use color_eyre::Result;
use cranefrick_assembler::{AssembledModule, AssemblerFlags};
use frick_ir::{AstParser as BrainParser, Compiler};
use ron::ser::PrettyConfig;
use serde::Serialize;
use tracing::{info, warn};
use tracing_error::ErrorLayer;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::{
	EnvFilter,
	fmt::{self, format::FmtSpan},
	prelude::*,
};
use tracing_tree::HierarchicalLayer;

use self::args::Args;

#[cfg(target_os = "windows")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<()> {
	let args = match Args::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};
	install_tracing(&args.output_path);
	color_eyre::install()?;

	let raw_data = fs::read_to_string(&args.file_path)?
		.chars()
		.filter(|c| matches!(c, '[' | ']' | '>' | '<' | '+' | '-' | ',' | '.'))
		.collect::<String>();

	let parser = BrainParser::new(raw_data.clone());

	let parsed = parser.parse()?;

	if parsed.is_empty() {
		return Ok(());
	}

	let mut compiler = Compiler::from_iter(parsed);

	serialize(&compiler, &args.output_path, "unoptimized")?;

	compiler.optimize();

	serialize(&compiler, &args.output_path, "optimized")?;

	let flags = get_flags(args.flags_path.as_deref());

	let module = AssembledModule::assemble(compiler, flags, &args.output_path)?;

	info!("running code");

	module.execute()?;

	Ok(())
}

fn install_tracing(folder_path: &Path) {
	_ = fs::remove_dir_all(folder_path);

	fs::create_dir_all(folder_path).unwrap();

	let log_file = fs::OpenOptions::new()
		.create(true)
		.write(true)
		.truncate(true)
		.open(folder_path.join("output.log"))
		.expect("failed to create log file");

	let json_log_file = fs::OpenOptions::new()
		.create(true)
		.truncate(true)
		.write(true)
		.open(folder_path.join("output.json"))
		.expect("failed to create json log file");

	let tree_log_file = fs::OpenOptions::new()
		.create(true)
		.truncate(true)
		.write(true)
		.open(folder_path.join("output.tree"))
		.expect("failed to create tree log file");

	let indicatif_layer = IndicatifLayer::new().with_progress_style(
		tracing_indicatif::style::ProgressStyle::with_template(
			"{span_child_prefix}{spinner} {span_name}({span_fields}) [{elapsed_precise}]",
		)
		.unwrap()
		.progress_chars("#>-"),
	);

	let file_layer = fmt::layer().with_ansi(false).with_writer(log_file);

	let fmt_layer = fmt::layer()
		.with_target(false)
		.with_writer(indicatif_layer.get_stderr_writer())
		.with_filter(env_filter());

	let json_file_layer = fmt::layer()
		.with_ansi(false)
		.json()
		.flatten_event(true)
		.with_span_events(FmtSpan::FULL)
		.with_writer(json_log_file);

	let tree_file_layer = HierarchicalLayer::new(2)
		.with_ansi(false)
		.with_bracketed_fields(true)
		.with_writer(tree_log_file);

	tracing_subscriber::registry()
		.with(json_file_layer)
		.with(file_layer)
		.with(fmt_layer)
		.with(indicatif_layer)
		.with(tree_file_layer)
		.with(ErrorLayer::default())
		.init();
}

fn env_filter() -> EnvFilter {
	EnvFilter::new("info,cranelift_jit=warn")
}

fn serialize<T: Serialize>(value: &T, folder_path: &Path, file_name: &str) -> Result<()> {
	serialize_as_ron(value, folder_path, file_name)?;

	serialize_as_s_expr(value, folder_path, file_name)
}

fn serialize_as_ron<T: Serialize>(value: &T, folder_path: &Path, file_name: &str) -> Result<()> {
	let mut output = String::new();
	let mut serializer = ron::Serializer::with_options(
		&mut output,
		Some(PrettyConfig::new().separate_tuple_members(true)),
		&ron::Options::default(),
	)?;

	value.serialize(&mut serializer)?;

	drop(serializer);

	fs::write(folder_path.join(format!("{file_name}.ron")), output)?;

	Ok(())
}

fn serialize_as_s_expr<T: Serialize>(value: &T, folder_path: &Path, file_name: &str) -> Result<()> {
	let file = fs::OpenOptions::new()
		.create(true)
		.truncate(true)
		.write(true)
		.open(folder_path.join(format!("{file_name}.s-expr")))?;

	let options = serde_lexpr::print::Options::elisp();

	serde_lexpr::to_writer_custom(file, value, options)?;

	Ok(())
}

fn get_flags(path: Option<&Path>) -> AssemblerFlags {
	if let Some(path) = path {
		let data = match fs::read(path) {
			Ok(data) => data,
			Err(e) => {
				warn!("error reading flags file: {e}");
				warn!("resorting to default flags");
				return AssemblerFlags::default();
			}
		};

		match toml::from_slice(&data) {
			Ok(flags) => flags,
			Err(e) => {
				warn!("error deserializing flags: {e}");
				warn!("resorting to default flags");
				AssemblerFlags::default()
			}
		}
	} else {
		AssemblerFlags::default()
	}
}
