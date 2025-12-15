mod error;
mod inner;
mod verify;

use frick_instructions::BrainInstruction;
use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};
use tracing::info;

pub use self::error::*;
use self::inner::{Pass, passes};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct InstructionsOptimizer {
	instrs: Vec<BrainInstruction>,
}

impl InstructionsOptimizer {
	pub fn new(instrs: impl IntoIterator<Item = BrainInstruction>) -> Self {
		Self {
			instrs: instrs.collect_to(),
		}
	}

	#[tracing::instrument("optimize instructions", skip(self))]
	pub fn run(&mut self) -> Result<(), InstructionsOptimizerError> {
		let mut iteration = 0;

		let mut progress = self.run_passes(iteration);

		while progress {
			iteration += 1;
			progress = self.run_passes(iteration);
		}

		info!(iterations = iteration);

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	fn run_passes(&mut self, iteration: usize) -> bool {
		let mut progress = false;

		self.run_each_pass(&mut progress);

		progress
	}

	fn run_each_pass(&mut self, progress: &mut bool) {
		*progress |= self.run_pass(passes::PointerRedundantLoadsPass);

		*progress |= self.run_pass(passes::StoreLoadsPass);

		*progress |= self.run_pass(passes::SimplifyMultiplicationPass);
	}

	fn run_pass<P: Pass>(&mut self, mut pass: P) -> bool {
		pass.run(self.instrs_mut())
	}

	pub const fn instrs_mut(&mut self) -> &mut Vec<BrainInstruction> {
		&mut self.instrs
	}
}
