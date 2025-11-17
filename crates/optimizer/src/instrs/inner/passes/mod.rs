mod pointer;

use frick_instructions::BrainInstruction;

pub use self::pointer::*;

pub trait AnalysisPass {
	fn run(&mut self, instrs: &[BrainInstruction]) -> bool;
}

pub trait Pass {
	fn run(&mut self, instrs: &mut Vec<BrainInstruction>) -> bool;

	fn run_analysis_passes(&mut self, instrs: &[BrainInstruction]) -> bool {
		let _ = instrs;
		false
	}
}
