use color_eyre::{Result, eyre::ContextCompat};
use inkwell::{context::Context, intrinsics::Intrinsic};

fn main() -> Result<()> {
	color_eyre::install()?;

	let context = Context::create();
	let module = context.create_module("intrinsics");

	let intrinsic = Intrinsic::find("llvm.assume").context("no intrinsic found")?;

	let _func = intrinsic
		.get_declaration(&module, &[])
		.context("no declaration found")?;

	module.print_to_stderr();

	Ok(())
}
