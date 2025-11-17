use frick_instructions::BrainInstruction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct InstructionsOptimizer {
	instrs: Vec<BrainInstruction>,
}
