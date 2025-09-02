mod ext;

use color_eyre::Result;
use hugr::{
	Hugr,
	algorithms::ComposablePass as _,
	builder::{
		DFGBuilder, Dataflow, DataflowHugr, DataflowSubContainer, HugrBuilder,
		inout_sig,
	},
	extension::prelude::*,
	ops::Value,
	types::Signature,
};
use hugr_core::export::export_hugr;
use hugr_model::v0::bumpalo::Bump;

fn main() -> Result<()> {
	color_eyre::install()?;

	let mut hugr = {
		let mut dfg_builder = DFGBuilder::new(inout_sig(vec![], vec![bool_t()]))?;

		let new_defn = {
			let mut mb = dfg_builder.module_root_builder();

			let fb = mb.define_function("helper_id", Signature::new_endo(bool_t()))?;

			let [f_inp] = fb.input_wires_arr();
			fb.finish_with_outputs([f_inp])
		}?;

		let new_decl = dfg_builder
			.module_root_builder()
			.declare("helper2", Signature::new_endo(bool_t()).into())?;

		let cst = dfg_builder.add_load_value(Value::true_val());

		let [c1] = dfg_builder
			.call(new_defn.handle(), &[], [cst])?
			.outputs_arr();

		let [c2] = dfg_builder.call(&new_decl, &[], [c1])?.outputs_arr();

		dfg_builder.finish_hugr_with_outputs([c2])
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
