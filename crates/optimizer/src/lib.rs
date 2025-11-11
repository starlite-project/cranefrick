#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;

mod inner;

use alloc::vec::Vec;

use frick_operations::BrainOperation;
use frick_utils::IntoIteratorExt as _;
use inner::{passes, run_peephole_pass};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Optimizer {
	ops: Vec<BrainOperation>,
}

impl Optimizer {
	pub fn new(ops: impl IntoIterator<Item = BrainOperation>) -> Self {
		Self {
			ops: ops.collect_to(),
		}
	}

	#[tracing::instrument("optimize operations", skip(self))]
	pub fn run(&mut self) {
		let mut iteration = 0;

		let mut progress = self.run_passes(iteration);

		while progress {
			iteration += 1;
			progress = self.run_passes(iteration);
		}

		info!(iterations = iteration);
	}

	#[tracing::instrument(skip(self))]
	fn run_passes(&mut self, iteration: usize) -> bool {
		let mut progress = false;

		self.run_each_pass(&mut progress);

		progress
	}

	fn run_each_pass(&mut self, progress: &mut bool) {
		*progress |= run_peephole_pass(self.ops_mut(), passes::optimize_consecutive_instructions);

		*progress |= run_peephole_pass(self.ops_mut(), passes::remove_comments);
	}

	#[must_use]
	pub const fn ops(&self) -> &Vec<BrainOperation> {
		&self.ops
	}

	pub const fn ops_mut(&mut self) -> &mut Vec<BrainOperation> {
		&mut self.ops
	}
}
