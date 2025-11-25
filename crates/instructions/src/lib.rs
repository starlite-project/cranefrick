#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;

use alloc::{vec, vec::Vec};
use core::ops::{Deref, DerefMut, Range};

use frick_operations::{BrainOperation, BrainOperationType};
use frick_spec::POINTER_SIZE;
use frick_utils::Convert as _;
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
		input_reg: Reg,
		output_reg: Reg,
	},
	StoreRegisterIntoCell {
		value_reg: Reg,
		pointer_reg: Reg,
	},
	StoreImmediateIntoRegister {
		imm: Imm,
		output_reg: Reg,
	},
	LoadTapePointerIntoRegister {
		output_reg: Reg,
	},
	StoreRegisterIntoTapePointer {
		input_reg: Reg,
	},
	CalculateTapeOffset {
		input_reg: Reg,
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
				BrainInstructionType::LoadTapePointerIntoRegister { output_reg: Reg(0) },
				BrainInstructionType::CalculateTapeOffset {
					input_reg: Reg(0),
					output_reg: Reg(1),
				},
				BrainInstructionType::LoadCellIntoRegister {
					input_reg: Reg(1),
					output_reg: Reg(2),
				},
				BrainInstructionType::StoreImmediateIntoRegister {
					imm: Imm::cell(value.unsigned_abs().convert::<u64>()),
					output_reg: Reg(3),
				},
				BrainInstructionType::PerformBinaryRegisterOperation {
					lhs_reg: Reg(2),
					rhs_reg: Reg(3),
					output_reg: Reg(4),
					op: if value.is_positive() {
						BinaryOperation::Add
					} else {
						BinaryOperation::Sub
					},
				},
				BrainInstructionType::StoreRegisterIntoCell {
					value_reg: Reg(4),
					pointer_reg: Reg(1),
				},
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::SetCell(value) => [
				BrainInstructionType::LoadTapePointerIntoRegister { output_reg: Reg(0) },
				BrainInstructionType::CalculateTapeOffset {
					input_reg: Reg(0),
					output_reg: Reg(1),
				},
				BrainInstructionType::StoreImmediateIntoRegister {
					output_reg: Reg(2),
					imm: Imm::cell(value.convert::<u64>()),
				},
				BrainInstructionType::StoreRegisterIntoCell {
					value_reg: Reg(2),
					pointer_reg: Reg(1),
				},
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
				BrainInstructionType::LoadTapePointerIntoRegister { output_reg: Reg(1) },
				BrainInstructionType::CalculateTapeOffset {
					input_reg: Reg(1),
					output_reg: Reg(2),
				},
				BrainInstructionType::StoreRegisterIntoCell {
					value_reg: Reg(0),
					pointer_reg: Reg(2),
				},
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::OutputCurrentCell => [
				BrainInstructionType::LoadTapePointerIntoRegister { output_reg: Reg(0) },
				BrainInstructionType::CalculateTapeOffset {
					input_reg: Reg(0),
					output_reg: Reg(1),
				},
				BrainInstructionType::LoadCellIntoRegister {
					input_reg: Reg(1),
					output_reg: Reg(2),
				},
				BrainInstructionType::OutputFromRegister { input_reg: Reg(2) },
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			BrainOperationType::DynamicLoop(ops) => {
				let mut output = [
					BrainInstructionType::StartLoop,
					BrainInstructionType::LoadTapePointerIntoRegister { output_reg: Reg(0) },
					BrainInstructionType::CalculateTapeOffset {
						input_reg: Reg(0),
						output_reg: Reg(1),
					},
					BrainInstructionType::LoadCellIntoRegister {
						input_reg: Reg(1),
						output_reg: Reg(2),
					},
					BrainInstructionType::StoreImmediateIntoRegister {
						output_reg: Reg(3),
						imm: Imm::CELL_ZERO,
					},
					BrainInstructionType::CompareRegisterToRegister {
						lhs_reg: Reg(2),
						rhs_reg: Reg(3),
						output_reg: Reg(4),
					},
					BrainInstructionType::JumpIf { input_reg: Reg(4) },
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
pub struct Imm {
	value: u64,
	size: u32,
}

impl Imm {
	pub const CELL_ZERO: Self = Self::cell(0);

	#[must_use]
	pub const fn new(value: u64, size: u32) -> Self {
		Self { value, size }
	}

	#[must_use]
	pub const fn pointer(value: u64) -> Self {
		Self::new(value, POINTER_SIZE as u32)
	}

	#[must_use]
	pub const fn cell(value: u64) -> Self {
		Self::new(value, 8)
	}

	#[must_use]
	pub const fn value(self) -> u64 {
		self.value
	}

	#[must_use]
	pub const fn size(self) -> u32 {
		self.size
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BinaryOperation {
	Add,
	Sub,
	BitwiseAnd,
}
