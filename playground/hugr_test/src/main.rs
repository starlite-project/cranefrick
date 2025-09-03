mod ext;

use color_eyre::Result;
use ext::{cx_gate, h_gate, measure};
use hugr::{
	Hugr,
	algorithms::ComposablePass as _,
	builder::{DFGBuilder, Dataflow, DataflowHugr, DataflowSubContainer, HugrBuilder, inout_sig},
	extension::prelude::*,
	ops::Value,
	types::Signature,
};
use hugr_core::export::export_hugr;
use hugr_model::v0::bumpalo::Bump;

fn main() -> Result<()> {
	color_eyre::install()?;

	let mut hugr = {
		let mut dfg_builder = DFGBuilder::new(inout_sig(
			vec![qb_t(), qb_t()],
			vec![qb_t(), qb_t(), bool_t()],
		))?;

		let [wire0, wire1] = dfg_builder.input_wires_arr();

		let h0 = dfg_builder.add_dataflow_op(h_gate(), vec![wire0])?;
		let h1 = dfg_builder.add_dataflow_op(h_gate(), vec![wire1])?;
		let cx = dfg_builder.add_dataflow_op(cx_gate(), h0.outputs().chain(h1.outputs()))?;
		let measure = dfg_builder.add_dataflow_op(measure(), cx.outputs().last())?;

		dfg_builder.finish_hugr_with_outputs(cx.outputs().take(1).chain(measure.outputs()))
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
