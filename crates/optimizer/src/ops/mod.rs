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
		*progress |= run_peephole_pass(self.ops_mut(), passes::optimize_consecutive_ops);
		*progress |= run_peephole_pass(self.ops_mut(), passes::optimize_set_cell);
		*progress |= run_loop_pass(self.ops_mut(), passes::optimize_clear_cell);
		*progress |= run_peephole_pass(self.ops_mut(), passes::optimize_output_value);
		*progress |= run_peephole_pass(self.ops_mut(), passes::optimize_output_cell);
		*progress |= run_peephole_pass(self.ops_mut(), passes::add_offsets);

		*progress |= run_loop_pass(self.ops_mut(), passes::optimize_move_cell_value);
		*progress |= run_peephole_pass(self.ops_mut(), passes::optimize_constant_moves);
		*progress |= run_peephole_pass(self.ops_mut(), passes::optimize_take_cell_value);

		*progress |= passes::fix_beginning_instructions(self.ops_mut());

		*progress |= run_peephole_pass(self.ops_mut(), passes::remove_unreachable_loops);
		*progress |= run_peephole_pass(self.ops_mut(), passes::remove_changes_before_input);
		*progress |= run_peephole_pass(self.ops_mut(), passes::remove_noop_ops);
		*progress |= run_peephole_pass(self.ops_mut(), passes::remove_redundant_offsets);

		*progress |= run_peephole_pass(self.ops_mut(), passes::unroll_constant_loop);
	}

	pub const fn ops(&self) -> &Vec<BrainOperation> {
		&self.ops
	}

	pub const fn ops_mut(&mut self) -> &mut Vec<BrainOperation> {
		&mut self.ops
	}
}
