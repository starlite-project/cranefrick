use std::{
	fs,
	path::{Path, PathBuf},
};

use clap::Parser;
use color_eyre::Result;
use cranefrick_assembler::{AssembledModule, AssemblerFlags};
use cranefrick_hlir::Parser as BrainParser;
use cranefrick_mlir::Compiler;
use ron::ser::PrettyConfig;
use serde::Serialize as _;
use tracing::{info, warn};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
	EnvFilter,
	fmt::{self, format::FmtSpan},
	prelude::*,
};

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

	let raw_data = fs::read_to_string(args.file_path)?;

	let parsed = BrainParser::new(&raw_data).parse::<Vec<_>>()?;

	let mut compiler = Compiler::from_iter(parsed.clone());

	serialize_compiler(&compiler, &args.output_path, "unoptimized")?;

	compiler.optimize();

	serialize_compiler(&compiler, &args.output_path, "optimized")?;

	let flags = get_flags(args.flags_path.as_deref());

	let module = AssembledModule::assemble(compiler, flags, &args.output_path)?;

	info!("running code");

	module.execute()?;

	Ok(())
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
	pub file_path: PathBuf,
	#[arg(short, long)]
	pub output_path: PathBuf,
	#[arg(short, long)]
	pub flags_path: Option<PathBuf>,
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

	let file_layer = fmt::layer().with_ansi(false).with_writer(log_file);

	let filter_layer = EnvFilter::new("info,cranelift_jit=warn");
	let fmt_layer = fmt::layer().with_target(false).with_filter(filter_layer);

	let json_file_layer = fmt::layer()
		.with_ansi(false)
		.json()
		.flatten_event(true)
		.with_span_events(FmtSpan::FULL)
		.with_writer(json_log_file);

	tracing_subscriber::registry()
		.with(json_file_layer)
		.with(file_layer)
		.with(fmt_layer)
		.with(ErrorLayer::default())
		.init();
}

fn serialize_compiler(comp: &Compiler, folder_path: &Path, file_name: &str) -> Result<()> {
	let mut output = String::new();
	let mut serializer = ron::Serializer::with_options(
		&mut output,
		Some(PrettyConfig::new().separate_tuple_members(true)),
		&ron::Options::default(),
	)?;

	comp.serialize(&mut serializer)?;

	// fs::write(format!("./out/{file_name}.ron"), output)?;
	fs::write(folder_path.join(format!("{file_name}.ron")), output)?;

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
