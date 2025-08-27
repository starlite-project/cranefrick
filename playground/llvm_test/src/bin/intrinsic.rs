use color_eyre::{Result, eyre::ContextCompat};
use inkwell::{AddressSpace, context::Context, intrinsics::Intrinsic};

fn main() -> Result<()> {
	color_eyre::install()?;

	let context = Context::create();
	let module = context.create_module("intrinsics");
	let builder = context.create_builder();
	let ptr_type = context.ptr_type(AddressSpace::default());
	let void_type = context.void_type();
	let i64_type = context.i64_type();

	let lifetime_start_intrinsic =
		Intrinsic::find("llvm.lifetime.start").context("no intrinsic found")?;

	let lifetime_start = lifetime_start_intrinsic
		.get_declaration(&module, &[ptr_type.into()])
		.context("no declaration found")?;

	let lifetime_end_intrinsic =
		Intrinsic::find("llvm.lifetime.end").context("no intrinsic found")?;

	let lifetime_end = lifetime_end_intrinsic
		.get_declaration(&module, &[ptr_type.into()])
		.context("no declaration found")?;

	let main = module.add_function("main", void_type.fn_type(&[], false), None);

	let entry = context.append_basic_block(main, "entry");
	builder.position_at_end(entry);

	let i64_alloca = builder.build_alloca(i64_type, "initial alloca")?;

	builder.build_call(lifetime_start, &[i64_alloca.into()], "lifetime start")?;

	builder.build_return(None)?;

	module.print_to_stderr();

	Ok(())
}
