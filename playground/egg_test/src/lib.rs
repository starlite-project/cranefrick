#![allow(clippy::wildcard_imports)]

use std::{error::Error, fmt::Display};

use color_eyre::Result;
use egg::*;

pub fn run_expr<L, N, CF>(
	expr: &str,
	rules: &[Rewrite<L, N>],
	cost_function: CF,
) -> Result<(String, String)>
where
	L: Display + FromOp + Language,
	<L as FromOp>::Error: Error + Send + Sync + 'static,
	N: Analysis<L> + Default,
	CF: CostFunction<L>,
{
	let start = expr.parse()?;

	let mut runner = Runner::default()
		.with_explanations_enabled()
		.with_explanation_length_optimization()
		.with_expr(&start)
		.run(rules);

	let end = {
		let extractor = Extractor::new(&runner.egraph, cost_function);

		let (.., best) = extractor.find_best(runner.roots[0]);

		best
	};

	let explanation = runner.explain_equivalence(&start, &end).get_flat_string();

	Ok((end.to_string(), explanation))
}
