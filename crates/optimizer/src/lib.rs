#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

use alloc::vec::Vec;
use core::mem;

use frick_instructions::{BrainInstruction, ToInstructions};
use frick_operations::BrainOperation;
use ops::OperationsOptimizer;

extern crate alloc;

mod instrs;
mod ops;

#[derive(Debug, Clone, Copy)]
pub struct Optimizer;

impl Optimizer {
	pub fn run(ops: impl IntoIterator<Item = BrainOperation>) -> Vec<BrainInstruction> {
		let mut ops_optimizer = OperationsOptimizer::new(ops);

		ops_optimizer.run();

		let finished_ops = mem::take(ops_optimizer.ops_mut());

		finished_ops
			.into_iter()
			.flat_map(|op| op.to_instructions())
			.collect()
	}
}
