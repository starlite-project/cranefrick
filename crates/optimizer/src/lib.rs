#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

use alloc::vec::Vec;

use frick_instructions::BrainInstruction;
use frick_operations::BrainOperation;

extern crate alloc;

mod instrs;
mod ops;

#[derive(Debug, Clone, Copy)]
pub struct Optimizer;

impl Optimizer {
	pub fn run(ops: impl IntoIterator<Item = BrainOperation>) -> Vec<BrainInstruction> {
		todo!()
	}
}
