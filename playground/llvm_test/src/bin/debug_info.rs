use color_eyre::{Report, Result};
use inkwell::{
	context::Context,
	debug_info::{
		AsDIScope, DIFlags, DIFlagsConstants as _, DWARFEmissionKind, DWARFSourceLanguage,
	},
	module::FlagBehavior,
	targets::{InitializationConfig, Target},
};

fn main() -> Result<()> {
	color_eyre::install()?;

	Target::initialize_native(&InitializationConfig::default()).map_err(Report::msg)?;
	let context = Context::create();
	let module = context.create_module("bin");

	let debug_metadata_version = context.i32_type().const_int(3, false);

	module.add_basic_value_flag(
		"Debug Info Version",
		FlagBehavior::Warning,
		debug_metadata_version,
	);

	let builder = context.create_builder();
	let (dibuilder, compile_unit) = module.create_debug_info_builder(
		true,
		DWARFSourceLanguage::C,
		"source_file",
		".",
		"my llvm compiler frontend",
		false,
		"",
		0,
		"",
		DWARFEmissionKind::Full,
		0,
		false,
		false,
		"",
		"",
	);

	let ditype = dibuilder
		.create_basic_type("type_name", 0, 0x00, DIFlags::PUBLIC)
		.map_err(Report::msg)?;

	let subroutine_type = dibuilder.create_subroutine_type(
		compile_unit.get_file(),
		Some(ditype.as_type()),
		&[],
		DIFlags::PUBLIC,
	);

	let func_scope = dibuilder.create_function(
		compile_unit.as_debug_info_scope(),
		"main",
		None,
		compile_unit.get_file(),
		0,
		subroutine_type,
		true,
		true,
		0,
		DIFlags::PUBLIC,
		false,
	);

	dibuilder.finalize();

	module.print_to_stderr();

	Ok(())
}
