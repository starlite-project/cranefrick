#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;

use alloc::{vec, vec::Vec};
use core::ops::{Deref, DerefMut, Range};

use frick_operations::{BrainOperation, BrainOperationType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BrainInstruction {
	instr: BrainInstructionType,
	#[serde(skip)]
	byte_offset: usize,
}

impl BrainInstruction {
	#[must_use]
	pub const fn new(instr: BrainInstructionType, byte_offset: usize) -> Self {
		Self { instr, byte_offset }
	}

	#[must_use]
	pub const fn instr(self) -> BrainInstructionType {
		self.instr
	}

	#[must_use]
	pub const fn byte_offset(self) -> usize {
		self.byte_offset
	}

	#[must_use]
	pub const fn span(self) -> Range<usize> {
		self.byte_offset()..self.byte_offset()
	}
}

impl Deref for BrainInstruction {
	type Target = BrainInstructionType;

	fn deref(&self) -> &Self::Target {
		&self.instr
	}
}

impl DerefMut for BrainInstruction {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.instr
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainInstructionType {
	LoadCellIntoRegister {
		output_reg: Reg,
	},
	StoreRegisterIntoCell {
		input_reg: Reg,
	},
	StoreImmediateIntoRegister {
		imm: u8,
		output_reg: Reg,
	},
	PerformBinaryRegisterOperation {
		lhs_reg: Reg,
		rhs_reg: Reg,
		output_reg: Reg,
		op: BinaryOperation,
	},
	InputIntoRegister {
		output_reg: Reg,
	},
	OutputFromRegister {
		input_reg: Reg,
	},
	LoadPointer,
	OffsetPointer {
		offset: i32,
	},
	StorePointer,
	StartLoop,
	EndLoop,
	CompareRegisterToRegister {
		lhs_reg: Reg,
		rhs_reg: Reg,
		output_reg: Reg,
	},
	JumpIf {
		input_reg: Reg,
	},
	JumpToHeader,
	NotImplemented,
}

pub trait ToInstructions {
	fn to_instructions(&self) -> Vec<BrainInstruction>;
}

impl ToInstructions for BrainOperation {
	fn to_instructions(&self) -> Vec<BrainInstruction> {
		match self.op() {
			&BrainOperationType::ChangeCell(value) => [
				BrainInstructionType::LoadPointer,
				BrainInstructionType::LoadCellIntoRegister { output_reg: Reg(0) },
				BrainInstructionType::StoreImmediateIntoRegister {
					output_reg: Reg(1),
					imm: value.unsigned_abs(),
				},
				BrainInstructionType::PerformBinaryRegisterOperation {
					lhs_reg: Reg(0),
					rhs_reg: Reg(1),
					output_reg: Reg(2),
					op: if value.is_negative() {
						BinaryOperation::Sub
					} else {
						BinaryOperation::Add
					},
				},
				BrainInstructionType::StoreRegisterIntoCell { input_reg: Reg(2) },
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::SetCell(value) => [
				BrainInstructionType::LoadPointer,
				BrainInstructionType::StoreImmediateIntoRegister {
					output_reg: Reg(0),
					imm: value,
				},
				BrainInstructionType::StoreRegisterIntoCell { input_reg: Reg(0) },
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::MovePointer(offset) => [
				BrainInstructionType::LoadPointer,
				BrainInstructionType::OffsetPointer { offset },
				BrainInstructionType::StorePointer,
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::InputIntoCell => [
				BrainInstructionType::InputIntoRegister { output_reg: Reg(0) },
				BrainInstructionType::LoadPointer,
				BrainInstructionType::StoreRegisterIntoCell { input_reg: Reg(0) },
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::OutputCurrentCell => [
				BrainInstructionType::LoadPointer,
				BrainInstructionType::LoadCellIntoRegister { output_reg: Reg(0) },
				BrainInstructionType::OutputFromRegister { input_reg: Reg(0) },
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			BrainOperationType::DynamicLoop(ops) => {
				let mut output = [
					BrainInstructionType::StartLoop,
					BrainInstructionType::LoadPointer,
					BrainInstructionType::LoadCellIntoRegister { output_reg: Reg(0) },
					BrainInstructionType::StoreImmediateIntoRegister {
						output_reg: Reg(1),
						imm: 0,
					},
					BrainInstructionType::CompareRegisterToRegister {
						lhs_reg: Reg(0),
						rhs_reg: Reg(1),
						output_reg: Reg(2),
					},
					BrainInstructionType::JumpIf { input_reg: Reg(2) },
				]
				.into_iter()
				.map(|x| BrainInstruction::new(x, self.span().start))
				.collect::<Vec<_>>();

				for op in ops {
					output.extend(op.to_instructions());
				}

				output.extend(
					[
						BrainInstructionType::JumpToHeader,
						BrainInstructionType::EndLoop,
					]
					.into_iter()
					.map(|x| BrainInstruction::new(x, self.span().end)),
				);

				output
			}
			BrainOperationType::Comment(..) => Vec::new(),
			_ => vec![BrainInstruction::new(
				BrainInstructionType::NotImplemented,
				self.span().start,
			)],
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BinaryOperation {
	Add,
	Sub,
}
