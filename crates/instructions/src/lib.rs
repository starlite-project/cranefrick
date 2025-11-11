#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;
use alloc::{vec, vec::Vec};

use frick_operations::{BrainOperation, BrainOperationType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrainInstruction {
	LoadCellIntoValue(u8),
	StoreValueIntoCell(u8),
	ChangeValueByConstant(u8, i8),
	OutputValue(u8),
	OffsetPointer(i32),
	StorePointer,
	StartLoop,
	EndLoop,
}

pub trait ToInstructions {
	fn to_instructions(&self) -> Vec<BrainInstruction>;
}

impl ToInstructions for BrainOperation {
	fn to_instructions(&self) -> Vec<BrainInstruction> {
		self.ty().to_instructions()
	}
}

impl ToInstructions for BrainOperationType {
	fn to_instructions(&self) -> Vec<BrainInstruction> {
		match self {
			&Self::ChangeCell(value) => vec![
				BrainInstruction::LoadCellIntoValue(0),
				BrainInstruction::ChangeValueByConstant(0, value),
				BrainInstruction::StoreValueIntoCell(0),
			],
			&Self::MovePointer(offset) => vec![
				BrainInstruction::OffsetPointer(offset),
				BrainInstruction::StorePointer,
			],
			Self::DynamicLoop(ops) => {
				let mut output = vec![BrainInstruction::StartLoop];

				for op in ops {
					output.extend(op.to_instructions());
				}

				output.push(BrainInstruction::EndLoop);

				output
			}
			_ => Vec::new(),
		}
	}
}
