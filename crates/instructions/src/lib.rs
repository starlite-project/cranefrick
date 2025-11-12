#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;
use alloc::{vec, vec::Vec};

use frick_operations::{BrainOperation, BrainOperationType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainInstruction {
	LoadCellIntoRegister(Reg),
	StoreRegisterIntoCell(Reg),
	ChangeRegisterByImmediate(Reg, i8),
	InputIntoRegister(Reg),
	OutputFromRegister(Reg),
	LoadPointer,
	OffsetPointer(i32),
	StorePointer,
	StartLoop,
	EndLoop,
	JumpIfZero(Reg),
	JumpIfNotZero(Reg),
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
				BrainInstruction::LoadPointer,
				BrainInstruction::LoadCellIntoRegister(Reg(0)),
				BrainInstruction::ChangeRegisterByImmediate(Reg(0), value),
				BrainInstruction::StoreRegisterIntoCell(Reg(0)),
			],
			&Self::MovePointer(offset) => vec![
				BrainInstruction::LoadPointer,
				BrainInstruction::OffsetPointer(offset),
				BrainInstruction::StorePointer,
			],
			&Self::InputIntoCell => vec![
				BrainInstruction::InputIntoRegister(Reg(0)),
				BrainInstruction::LoadPointer,
				BrainInstruction::StoreRegisterIntoCell(Reg(0)),
			],
			&Self::OutputCurrentCell => vec![
				BrainInstruction::LoadPointer,
				BrainInstruction::LoadCellIntoRegister(Reg(0)),
				BrainInstruction::OutputFromRegister(Reg(0)),
			],
			Self::DynamicLoop(ops) => {
				let mut output = vec![
					BrainInstruction::StartLoop,
					BrainInstruction::LoadPointer,
					BrainInstruction::LoadCellIntoRegister(Reg(0)),
					BrainInstruction::JumpIfZero(Reg(0)),
				];

				for op in ops {
					output.extend(op.to_instructions());
				}

				output.extend([
					BrainInstruction::LoadPointer,
					BrainInstruction::LoadCellIntoRegister(Reg(0)),
					BrainInstruction::JumpIfNotZero(Reg(0)),
					BrainInstruction::EndLoop,
				]);

				output
			}
			_ => Vec::new(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Reg(pub usize);

impl From<usize> for Reg {
	fn from(value: usize) -> Self {
		Self(value)
	}
}

impl From<Reg> for usize {
	fn from(value: Reg) -> Self {
		value.0
	}
}
