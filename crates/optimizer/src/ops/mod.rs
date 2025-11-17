mod inner;

use frick_operations::BrainOperation;
use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};
use tracing::info;

use self::inner::{passes, run_loop_pass, run_peephole_pass};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct OperationsOptimizer {
	ops: Vec<BrainOperation>,
}

impl OperationsOptimizer {
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

		*progress |= run_peephole_pass(self.ops_mut(), passes::optimize_set_cell_instruction);
		*progress |= run_loop_pass(self.ops_mut(), passes::optimize_clear_cell_instruction);

		*progress |= passes::fix_beginning_instructions(self.ops_mut());

		*progress |= run_peephole_pass(self.ops_mut(), passes::remove_unreachable_loops);

		*progress |= run_peephole_pass(self.ops_mut(), passes::remove_comments);
	}

	pub const fn ops(&self) -> &Vec<BrainOperation> {
		&self.ops
	}

	pub const fn ops_mut(&mut self) -> &mut Vec<BrainOperation> {
		&mut self.ops
	}
}
