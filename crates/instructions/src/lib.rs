#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;

mod helpers;

use alloc::{vec, vec::Vec};
use core::{
	mem,
	ops::{Deref, DerefMut, Range},
};

use frick_operations::{BrainOperation, BrainOperationType, CellOffsetOptions};
use frick_spec::{POINTER_SIZE, TAPE_SIZE};
use frick_types::{BinaryOperation, Bool, Int, Pointer, Register};
use frick_utils::Convert as _;
use serde::{Deserialize, Serialize};

use self::helpers::LoadCellInformation;

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
		pointer_reg: Register<Pointer>,
		output_reg: Register<Int>,
	},
	StoreRegisterIntoCell {
		value_reg: Register<Int>,
		pointer_reg: Register<Pointer>,
	},
	StoreImmediateIntoRegister {
		imm: Imm,
		output_reg: Register<Int>,
	},
	LoadTapePointerIntoRegister {
		output_reg: Register<Int>,
	},
	StoreRegisterIntoTapePointer {
		input_reg: Register<Int>,
	},
	CalculateTapeOffset {
		tape_pointer_reg: Register<Int>,
		output_reg: Register<Pointer>,
	},
	PerformBinaryRegisterOperation {
		lhs_reg: Register<Int>,
		rhs_reg: Register<Int>,
		output_reg: Register<Int>,
		op: BinaryOperation,
	},
	InputIntoRegister {
		output_reg: Register<Int>,
	},
	OutputFromRegister {
		input_reg: Register<Int>,
	},
	StartLoop,
	EndLoop,
	CompareRegisterToRegister {
		lhs_reg: Register<Int>,
		rhs_reg: Register<Int>,
		output_reg: Register<Bool>,
	},
	JumpIf {
		input_reg: Register<Bool>,
	},
	JumpToHeader,
	NotImplemented,
}

impl BrainInstructionType {
	#[must_use]
	pub fn uses_register(self, reg: usize) -> bool {
		match self {
			Self::LoadCellIntoRegister {
				pointer_reg,
				output_reg: int_reg,
			}
			| Self::StoreRegisterIntoCell {
				value_reg: int_reg,
				pointer_reg,
			}
			| Self::CalculateTapeOffset {
				tape_pointer_reg: int_reg,
				output_reg: pointer_reg,
			} => pointer_reg == reg || int_reg == reg,
			Self::StoreImmediateIntoRegister {
				output_reg: int_reg,
				..
			}
			| Self::LoadTapePointerIntoRegister {
				output_reg: int_reg,
			}
			| Self::StoreRegisterIntoTapePointer { input_reg: int_reg }
			| Self::InputIntoRegister {
				output_reg: int_reg,
			}
			| Self::OutputFromRegister { input_reg: int_reg } => int_reg == reg,
			Self::PerformBinaryRegisterOperation {
				lhs_reg,
				rhs_reg,
				output_reg,
				..
			} => lhs_reg == reg || rhs_reg == reg || output_reg == reg,
			Self::CompareRegisterToRegister {
				lhs_reg,
				rhs_reg,
				output_reg,
			} => lhs_reg == reg || rhs_reg == reg || output_reg == reg,
			Self::JumpIf { input_reg } => input_reg == reg,
			_ => false,
		}
	}
}

pub trait ToInstructions {
	fn to_instructions(&self) -> Vec<BrainInstruction>;
}

impl ToInstructions for BrainOperation {
	fn to_instructions(&self) -> Vec<BrainInstruction> {
		match self.op() {
			&BrainOperationType::IncrementCell(CellOffsetOptions { value, offset }) => {
				let (load_cell_info, mut instrs) = LoadCellInformation::create(offset, 0, None);

				instrs.extend([
					BrainInstructionType::StoreImmediateIntoRegister {
						imm: Imm::cell(value.convert::<u64>()),
						output_reg: Register::new(load_cell_info.instr_offset),
					},
					BrainInstructionType::PerformBinaryRegisterOperation {
						lhs_reg: load_cell_info.cell_reg,
						rhs_reg: Register::new(load_cell_info.instr_offset),
						output_reg: Register::new(load_cell_info.instr_offset + 1),
						op: BinaryOperation::Add,
					},
					BrainInstructionType::StoreRegisterIntoCell {
						value_reg: Register::new(load_cell_info.instr_offset + 1),
						pointer_reg: load_cell_info.pointer_reg,
					},
				]);

				instrs
					.into_iter()
					.map(|i| BrainInstruction::new(i, self.span().start))
					.collect()
			}
			&BrainOperationType::DecrementCell(CellOffsetOptions { value, offset }) => {
				let (load_cell_info, mut instrs) = LoadCellInformation::create(offset, 0, None);

				instrs.extend([
					BrainInstructionType::StoreImmediateIntoRegister {
						imm: Imm::cell(value.convert::<u64>()),
						output_reg: Register::new(load_cell_info.instr_offset),
					},
					BrainInstructionType::PerformBinaryRegisterOperation {
						lhs_reg: load_cell_info.cell_reg,
						rhs_reg: Register::new(load_cell_info.instr_offset),
						output_reg: Register::new(load_cell_info.instr_offset + 1),
						op: BinaryOperation::Sub,
					},
					BrainInstructionType::StoreRegisterIntoCell {
						value_reg: Register::new(load_cell_info.instr_offset + 1),
						pointer_reg: load_cell_info.pointer_reg,
					},
				]);

				instrs
					.into_iter()
					.map(|i| BrainInstruction::new(i, self.span().start))
					.collect()
			}
			&BrainOperationType::SetCell(CellOffsetOptions { value, offset: 0 }) => [
				BrainInstructionType::LoadTapePointerIntoRegister {
					output_reg: Register::new(0),
				},
				BrainInstructionType::CalculateTapeOffset {
					tape_pointer_reg: Register::new(0),
					output_reg: Register::new(1),
				},
				BrainInstructionType::StoreImmediateIntoRegister {
					output_reg: Register::new(2),
					imm: Imm::cell(value.convert::<u64>()),
				},
				BrainInstructionType::StoreRegisterIntoCell {
					value_reg: Register::new(2),
					pointer_reg: Register::new(1),
				},
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::SetCell(CellOffsetOptions { value, offset }) => [
				BrainInstructionType::LoadTapePointerIntoRegister {
					output_reg: Register::new(0),
				},
				BrainInstructionType::StoreImmediateIntoRegister {
					imm: Imm::pointer(offset.unsigned_abs().convert::<u64>()),
					output_reg: Register::new(1),
				},
				BrainInstructionType::PerformBinaryRegisterOperation {
					lhs_reg: Register::new(0),
					rhs_reg: Register::new(1),
					output_reg: Register::new(2),
					op: if offset.is_positive() {
						BinaryOperation::Add
					} else {
						BinaryOperation::Sub
					},
				},
				BrainInstructionType::StoreImmediateIntoRegister {
					imm: Imm::pointer(TAPE_SIZE as u64 - 1),
					output_reg: Register::new(3),
				},
				BrainInstructionType::PerformBinaryRegisterOperation {
					lhs_reg: Register::new(2),
					rhs_reg: Register::new(3),
					output_reg: Register::new(4),
					op: BinaryOperation::BitwiseAnd,
				},
				BrainInstructionType::CalculateTapeOffset {
					tape_pointer_reg: Register::new(4),
					output_reg: Register::new(5),
				},
				BrainInstructionType::StoreImmediateIntoRegister {
					imm: Imm::cell(value.convert::<u64>()),
					output_reg: Register::new(6),
				},
				BrainInstructionType::StoreRegisterIntoCell {
					value_reg: Register::new(6),
					pointer_reg: Register::new(5),
				},
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::MovePointer(offset) => [
				BrainInstructionType::LoadTapePointerIntoRegister {
					output_reg: Register::new(0),
				},
				BrainInstructionType::StoreImmediateIntoRegister {
					imm: Imm::pointer(offset.unsigned_abs().convert::<u64>()),
					output_reg: Register::new(1),
				},
				BrainInstructionType::PerformBinaryRegisterOperation {
					lhs_reg: Register::new(0),
					rhs_reg: Register::new(1),
					output_reg: Register::new(2),
					op: if offset.is_positive() {
						BinaryOperation::Add
					} else {
						BinaryOperation::Sub
					},
				},
				BrainInstructionType::StoreImmediateIntoRegister {
					imm: Imm::pointer(TAPE_SIZE as u64 - 1),
					output_reg: Register::new(3),
				},
				BrainInstructionType::PerformBinaryRegisterOperation {
					lhs_reg: Register::new(2),
					rhs_reg: Register::new(3),
					output_reg: Register::new(4),
					op: BinaryOperation::BitwiseAnd,
				},
				BrainInstructionType::StoreRegisterIntoTapePointer {
					input_reg: Register::new(4),
				},
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::MoveCellValue(CellOffsetOptions { value, offset }) => {
				let (current_cell_info, mut instrs) = LoadCellInformation::create(0, 0, None);

				instrs.extend([
					BrainInstructionType::StoreImmediateIntoRegister {
						imm: Imm::CELL_ZERO,
						output_reg: Register::new(current_cell_info.instr_offset),
					},
					BrainInstructionType::StoreRegisterIntoCell {
						value_reg: Register::new(current_cell_info.instr_offset),
						pointer_reg: current_cell_info.pointer_reg,
					},
					BrainInstructionType::StoreImmediateIntoRegister {
						imm: Imm::cell(value.convert::<u64>()),
						output_reg: Register::new(current_cell_info.instr_offset + 1),
					},
					BrainInstructionType::PerformBinaryRegisterOperation {
						lhs_reg: current_cell_info.cell_reg,
						rhs_reg: Register::new(current_cell_info.instr_offset + 1),
						output_reg: Register::new(current_cell_info.instr_offset + 2),
						op: BinaryOperation::Mul,
					},
				]);

				let (other_cell_info, mut other_cell_instrs) = LoadCellInformation::create(
					offset,
					current_cell_info.instr_offset + 3,
					Some(current_cell_info.tape_pointer_reg),
				);

				instrs.append(&mut other_cell_instrs);

				instrs.extend([
					BrainInstructionType::PerformBinaryRegisterOperation {
						lhs_reg: Register::new(current_cell_info.instr_offset + 2),
						rhs_reg: other_cell_info.cell_reg,
						output_reg: Register::new(other_cell_info.instr_offset),
						op: BinaryOperation::Add,
					},
					BrainInstructionType::StoreRegisterIntoCell {
						value_reg: Register::new(other_cell_info.instr_offset),
						pointer_reg: other_cell_info.pointer_reg,
					},
				]);

				instrs
					.into_iter()
					.map(|i| BrainInstruction::new(i, self.span().start))
					.collect()
			}
			&BrainOperationType::InputIntoCell => [
				BrainInstructionType::InputIntoRegister {
					output_reg: Register::new(0),
				},
				BrainInstructionType::LoadTapePointerIntoRegister {
					output_reg: Register::new(1),
				},
				BrainInstructionType::CalculateTapeOffset {
					tape_pointer_reg: Register::new(1),
					output_reg: Register::new(2),
				},
				BrainInstructionType::StoreRegisterIntoCell {
					value_reg: Register::new(0),
					pointer_reg: Register::new(2),
				},
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::OutputCurrentCell => [
				BrainInstructionType::LoadTapePointerIntoRegister {
					output_reg: Register::new(0),
				},
				BrainInstructionType::CalculateTapeOffset {
					tape_pointer_reg: Register::new(0),
					output_reg: Register::new(1),
				},
				BrainInstructionType::LoadCellIntoRegister {
					pointer_reg: Register::new(1),
					output_reg: Register::new(2),
				},
				BrainInstructionType::OutputFromRegister {
					input_reg: Register::new(2),
				},
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::OutputValue(value) => [
				BrainInstructionType::StoreImmediateIntoRegister {
					imm: Imm::cell(value.convert::<u64>()),
					output_reg: Register::new(0),
				},
				BrainInstructionType::OutputFromRegister {
					input_reg: Register::new(0),
				},
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			BrainOperationType::DynamicLoop(ops) => {
				let mut output = [
					BrainInstructionType::StartLoop,
					BrainInstructionType::LoadTapePointerIntoRegister {
						output_reg: Register::new(0),
					},
					BrainInstructionType::CalculateTapeOffset {
						tape_pointer_reg: Register::new(0),
						output_reg: Register::new(1),
					},
					BrainInstructionType::LoadCellIntoRegister {
						pointer_reg: Register::new(1),
						output_reg: Register::new(2),
					},
					BrainInstructionType::StoreImmediateIntoRegister {
						output_reg: Register::new(3),
						imm: Imm::CELL_ZERO,
					},
					BrainInstructionType::CompareRegisterToRegister {
						lhs_reg: Register::new(2),
						rhs_reg: Register::new(3),
						output_reg: Register::new(4),
					},
					BrainInstructionType::JumpIf {
						input_reg: Register::new(4),
					},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Imm {
	value: u64,
	size: ImmSize,
}

impl Imm {
	pub const CELL_ZERO: Self = Self::cell(0);

	#[must_use]
	pub const fn new(value: u64, size: ImmSize) -> Self {
		Self { value, size }
	}

	#[must_use]
	pub const fn pointer(value: u64) -> Self {
		Self::new(value, ImmSize::Pointer)
	}

	#[must_use]
	pub const fn cell(value: u64) -> Self {
		Self::new(value, ImmSize::Cell)
	}

	#[must_use]
	pub const fn value(self) -> u64 {
		self.value
	}

	#[must_use]
	pub const fn size(self) -> u32 {
		match self.size {
			ImmSize::Cell => 8,
			ImmSize::Pointer => POINTER_SIZE as u32,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImmSize {
	Cell,
	Pointer,
}
