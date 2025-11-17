#![cfg_attr(docsrs, feature(doc_cfg))]

mod instrs;
mod ops;

use std::{mem, path::Path};

use frick_instructions::{BrainInstruction, ToInstructions};
use frick_operations::BrainOperation;

use self::ops::OperationsOptimizer;

#[derive(Debug, Clone, Copy)]
pub struct Optimizer;

impl Optimizer {
	pub fn run(
		ops: impl IntoIterator<Item = BrainOperation>,
		output_path: &Path,
	) -> Vec<BrainInstruction> {
		let mut ops_optimizer = OperationsOptimizer::new(ops);

		frick_serialize::serialize(&ops_optimizer, output_path, "unoptimized.ops").unwrap();

		ops_optimizer.run();

		frick_serialize::serialize(&ops_optimizer, output_path, "optimized.ron").unwrap();

		let finished_ops = mem::take(ops_optimizer.ops_mut());

		finished_ops
			.into_iter()
			.flat_map(|op| op.to_instructions())
			.collect()
	}
}
