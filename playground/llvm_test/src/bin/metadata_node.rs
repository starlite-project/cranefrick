use color_eyre::{Report, Result};
use inkwell::context::Context;

fn main() -> Result<()> {
	let context = Context::create();
	let module = context.create_module("my_mod");
	let builder = context.create_builder();

    let i32_type = context.i32_type();

    let md_node = context.metadata_node(&[]);



    module.print_to_stderr();

	Ok(())
}
