#![cfg_attr(docsrs, feature(doc_cfg))]

mod error;
mod instrs;
mod ops;

use std::{mem, path::Path};

use frick_instructions::{BrainInstruction, ToInstructions};
use frick_operations::BrainOperation;

pub use self::{error::OptimizerError, instrs::InstructionsOptimizerError};
use self::{instrs::InstructionsOptimizer, ops::OperationsOptimizer};

#[derive(Debug, Clone, Copy)]
pub struct Optimizer;

impl Optimizer {
	pub fn run(
		ops: impl IntoIterator<Item = BrainOperation>,
		output_path: &Path,
	) -> Result<Vec<BrainInstruction>, OptimizerError> {
		let mut ops_optimizer = OperationsOptimizer::new(ops);

		frick_serialize::serialize(&ops_optimizer, output_path, "unoptimized.ops")?;

		{
			let raw_instrs_optimizer = InstructionsOptimizer::new(
				ops_optimizer
					.ops()
					.iter()
					.flat_map(ToInstructions::to_instructions),
			);

			frick_serialize::serialize(
				&raw_instrs_optimizer,
				output_path,
				"unoptimized.ops.instrs",
			)?;
		}

		ops_optimizer.run();

		frick_serialize::serialize(&ops_optimizer, output_path, "optimized.ops")?;

		let finished_ops = mem::take(ops_optimizer.ops_mut());

		let mut instrs_optimizer = InstructionsOptimizer::new(
			finished_ops.into_iter().flat_map(|op| op.to_instructions()),
		);

		frick_serialize::serialize(&instrs_optimizer, output_path, "unoptimized.instrs")?;

		instrs_optimizer.run()?;

		frick_serialize::serialize(&instrs_optimizer, output_path, "optimized.instrs")?;

		Ok(mem::take(instrs_optimizer.instrs_mut()))
	}
}
