use color_eyre::Result;
use inkwell::{
	attributes::{Attribute, AttributeLoc},
	context::Context,
};

fn main() -> Result<()> {
	color_eyre::install()?;

	let context = Context::create();
	let module = context.create_module("attributes");
	let builder = context.create_builder();

	let i32_type = context.i32_type();
	let fn_type = i32_type.fn_type(&[i32_type.into()], false);
	let function = module.add_function("foo", fn_type, None);

	let attribute = context.create_enum_attribute(Attribute::get_named_enum_kind_id("nounwind"), 0);

	function.add_attribute(AttributeLoc::Function, attribute);

	let basic_block = context.append_basic_block(function, "entry");
	builder.position_at_end(basic_block);

	module.print_to_stderr();

	Ok(())
}
