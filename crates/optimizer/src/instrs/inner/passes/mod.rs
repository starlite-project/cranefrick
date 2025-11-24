mod pointer;

use frick_instructions::BrainInstruction;

pub use self::pointer::*;

pub trait Pass {
	fn run(&mut self, instrs: &mut Vec<BrainInstruction>) -> bool;
}
