use frick_instructions::{BrainInstruction, BrainInstructionType};

use crate::instrs::inner::Pass;

pub struct SimplifyMultiplicationPass;

impl Pass for SimplifyMultiplicationPass {
	// TODO: figure out how to get the value at the register we need it at
	fn run(&mut self, instrs: &mut Vec<BrainInstruction>) -> bool {
		let mut changed_any = false;

		changed_any
	}
}
