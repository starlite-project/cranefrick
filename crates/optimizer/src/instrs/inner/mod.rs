pub mod passes;

use frick_instructions::BrainInstruction;

pub trait Pass {
	fn run(&mut self, instrs: &mut Vec<BrainInstruction>) -> bool;
}

pub trait Analyzer {
	fn run(&mut self, instrs: &[BrainInstruction]) -> bool;
}
