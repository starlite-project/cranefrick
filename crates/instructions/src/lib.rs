#![cfg_attr(docsrs, feature(doc_cfg))]

use frick_operations::{BrainOperation, BrainOperationType};

pub enum BrainInstruction {
	LoadCell,
	StoreCell,
	StartLoop,
	EndLoop,
}

pub trait IntoInstructions {
	fn to_instructions(&self) -> Vec<BrainInstruction>;
}

impl IntoInstructions for BrainOperation {
	fn to_instructions(&self) -> Vec<BrainInstruction> {
		self.ty().to_instructions()
	}
}

impl IntoInstructions for BrainOperationType {
	fn to_instructions(&self) -> Vec<BrainInstruction> {
		Vec::new()
	}
}
