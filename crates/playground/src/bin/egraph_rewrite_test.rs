use std::str::FromStr;

use anyhow::Result;
use egg::{rewrite as rw, *};

define_language! {
	enum BfLang {
		Add(i32),
		Move(i32),
	}
}

impl FromStr for BfLang {
	type Err = String;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(match s {
			"+" => Self::Add(1),
			">" => Self::Move(1),
			"-" => Self::Add(-1),
			"<" => Self::Move(-1),
			c => return Err(format!("unknown opcode {c}")),
		})
	}
}

fn main() -> Result<()> {
	let rules: &[Rewrite<BfLang, ()>] = &[];

	let expr = "+++>>+++".parse()?;

	let mut runner = Runner::default().with_expr(&expr).run(rules);

	let extractor = Extractor::new(&runner.egraph, AstSize);

	let (best_cost, best_root) = extractor.find_best(runner.roots[0]);

	println!("{}", runner.explain_equivalence(&expr, &best_root));

	Ok(())
}
