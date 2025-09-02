use color_eyre::Result;
use hugr::{
	Hugr,
	algorithms::ComposablePass as _,
	builder::{Dataflow, DataflowSubContainer, HugrBuilder, ModuleBuilder},
	extension::prelude::*,
	std_extensions::logic::LogicOp,
	types::Signature,
};
use hugr_core::export::export_hugr;
use hugr_model::v0::bumpalo::Bump;

fn main() -> Result<()> {
	color_eyre::install()?;

	let mut hugr = {
		let mut module_builder = ModuleBuilder::new();

		let _dfg_handle = {
			let mut dfg = module_builder.define_function("main", Signature::new_endo(bool_t()))?;

			let [w] = dfg.input_wires_arr();

			let [w] = dfg.add_dataflow_op(LogicOp::Not, [w])?.outputs_arr();

			dfg.finish_with_outputs([w])
		}?;

		let _circuit_handle = {
			let mut dfg = module_builder
				.define_function("circuit", Signature::new_endo(vec![bool_t(), bool_t()]))?;
			let mut circuit = dfg.as_circuit(dfg.input_wires());

			circuit
				.append(LogicOp::Not, [0])?
				.append(LogicOp::Not, [1])?;

			let outputs = circuit.finish();
			dfg.finish_with_outputs(outputs)
		}?;

		module_builder.finish_hugr()
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
