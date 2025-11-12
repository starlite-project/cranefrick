mod args;

use std::{fs, path::Path};

use clap::Parser as _;
use color_eyre::Result;
use frick_assembler::Assembler;
use frick_instructions::ToInstructions as _;
use frick_optimizer::Optimizer;
use ron::ser::PrettyConfig;
use serde::Serialize;
use tracing_error::ErrorLayer;
use tracing_indicatif::{IndicatifLayer, filter::IndicatifFilter, style::ProgressStyle};
use tracing_subscriber::{
	EnvFilter,
	fmt::{self, format::FmtSpan},
	prelude::*,
};

use self::args::Args;

#[cfg(target_os = "windows")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "heap_profiling")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> Result<()> {
	#[cfg(feature = "heap_profiling")]
	let _heap_profiler = dhat::Profiler::new_heap();

	let args = match Args::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};
	install_tracing(args.output_path());
	color_eyre::install()?;

	let operations = frick_operations::parse(args.file_path())?;

	if operations.is_empty() {
		tracing::warn!("no program parsed");

		return Ok(());
	}

	let mut optimizer = Optimizer::new(operations);

	serialize(
		&optimizer.ops().iter().map(|x| x.op()).collect::<Vec<_>>(),
		args.output_path(),
		"unoptimized.ops",
	)?;

	serialize(
		&optimizer
			.to_instructions()
			.iter()
			.map(|x| x.instr())
			.collect::<Vec<_>>(),
		args.output_path(),
		"unoptimized.instrs",
	)?;

	optimizer.run();

	serialize(
		&optimizer.ops().iter().map(|x| x.op()).collect::<Vec<_>>(),
		args.output_path(),
		"optimized.ops",
	)?;

	serialize(
		&optimizer
			.to_instructions()
			.iter()
			.map(|x| x.instr())
			.collect::<Vec<_>>(),
		args.output_path(),
		"optimized.instrs",
	)?;

	let assembler = match args.passes_path() {
		None => Assembler::new("default<O0>".to_owned(), args.file_path().to_owned()),
		Some(passes_path) => {
			let passes = fs::read_to_string(passes_path)?;

			Assembler::new(
				passes
					.lines()
					.map(|l| l.trim())
					.collect::<Vec<_>>()
					.join(","),
				args.file_path().to_owned(),
			)
		}
	};

	let module = assembler.assemble(optimizer.ops(), args.output_path())?;

	tracing::info!("finished assembling module");

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

	let indicatif_layer = IndicatifLayer::new().with_progress_style(
		ProgressStyle::with_template(
			"{span_child_prefix}{spinner} {span_name}({span_fields}) [{elapsed_precise}]",
		)
		.unwrap()
		.progress_chars("#>-"),
	);

	let file_layer = fmt::layer()
		.with_target(false)
		.with_ansi(false)
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.with_writer(log_file);

	let fmt_layer = fmt::layer()
		.with_target(false)
		.with_writer(indicatif_layer.get_stderr_writer())
		.with_filter(env_filter());

	tracing_subscriber::registry()
		.with(file_layer)
		.with(fmt_layer)
		.with(indicatif_layer.with_filter(IndicatifFilter::new(false)))
		.with(ErrorLayer::default())
		.init();
}

fn env_filter() -> EnvFilter {
	EnvFilter::new("debug")
}

fn serialize<T: Serialize>(value: &T, folder_path: &Path, file_name: &str) -> Result<()> {
	serialize_as_ron(value, folder_path, file_name)
}

fn serialize_as_ron<T: Serialize>(value: &T, folder_path: &Path, file_name: &str) -> Result<()> {
	let mut output = String::new();
	let mut serializer = ron::Serializer::with_options(
		&mut output,
		Some(PrettyConfig::new().separate_tuple_members(true)),
		&ron::Options::default().without_recursion_limit(),
	)?;

	value.serialize(&mut serializer)?;

	drop(serializer);

	fs::write(folder_path.join(format!("{file_name}.ron")), output)?;

	Ok(())
}
