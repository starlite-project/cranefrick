mod ext;

use color_eyre::Result;
use ext::inc_op;
use hugr::{
	Hugr,
	algorithms::ComposablePass as _,
	builder::{DFGBuilder, Dataflow, DataflowSubContainer, HugrBuilder, ModuleBuilder, inout_sig},
	extension::prelude::*,
	std_extensions::logic::LogicOp,
	types::Signature,
};
use hugr_core::export::export_hugr;
use hugr_model::v0::bumpalo::Bump;

fn main() -> Result<()> {
	color_eyre::install()?;

	let mut hugr = {
		let mut dfg_builder = DFGBuilder::new(inout_sig(vec![], vec![]))?;

		dfg_builder.finish_hugr()
	}?;

	print_hugr(&hugr);

	let pass = hugr::algorithms::LinearizeArrayPass::default();

	pass.run(&mut hugr)?;

	print_hugr(&hugr);

	Ok(())
}

fn print_hugr(hugr: &Hugr) {
	let bump = Bump::new();

	let exported_hugr = export_hugr(hugr, &bump);

	let Some(exported_ast) = exported_hugr.as_ast().map(|ast| ast.to_string()) else {
		return;
	};

	println!("Hugr:");
	println!("{exported_ast}");
}
