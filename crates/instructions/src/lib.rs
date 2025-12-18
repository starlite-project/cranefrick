#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;

mod helpers;

use alloc::{vec, vec::Vec};
use core::ops::{Deref, DerefMut, Range};

use frick_operations::{BrainOperation, BrainOperationType, CellOffsetOptions};
use frick_types::{Any, BinaryOperation, Bool, Immediate, Int, Pointer, RegOrImm, Register};
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
	#[deprecated]
	StoreRegisterIntoCell {
		value_reg: Register<Int>,
		pointer_reg: Register<Pointer>,
	},
	StoreValueIntoCell {
		value: RegOrImm<Int>,
		pointer_reg: Register<Pointer>,
	},
	StoreImmediateIntoRegister {
		imm: Immediate,
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
	#[deprecated]
	PerformBinaryRegisterOperation {
		lhs_reg: Register<Int>,
		rhs_reg: Register<Int>,
		output_reg: Register<Int>,
		op: BinaryOperation,
	},
	PerformBinaryValueOperation {
		lhs: RegOrImm<Int>,
		rhs: RegOrImm<Int>,
		output_reg: Register<Int>,
		op: BinaryOperation,
	},
	DuplicateRegister {
		input_reg: Register<Any>,
		output_reg: Register<Any>,
	},
	InputIntoRegister {
		output_reg: Register<Int>,
	},
	OutputFromRegister {
		input_reg: Register<Int>,
	},
	StartLoop,
	EndLoop,
	#[deprecated]
	CompareRegisterToRegister {
		lhs_reg: Register<Int>,
		rhs_reg: Register<Int>,
		output_reg: Register<Bool>,
	},
	CompareValues {
		lhs: RegOrImm<Int>,
		rhs: RegOrImm<Int>,
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
	#[allow(deprecated)]
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
					BrainInstructionType::PerformBinaryValueOperation {
						lhs: RegOrImm::Reg(load_cell_info.cell_reg),
						rhs: RegOrImm::Imm(Immediate::cell(value.convert::<u64>())),
						output_reg: Register::new(load_cell_info.instr_offset),
						op: BinaryOperation::Add,
					},
					BrainInstructionType::StoreValueIntoCell {
						value: RegOrImm::Reg(Register::new(load_cell_info.instr_offset)),
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
					BrainInstructionType::PerformBinaryValueOperation {
						lhs: RegOrImm::Reg(load_cell_info.cell_reg),
						rhs: RegOrImm::Imm(Immediate::cell(value.convert::<u64>())),
						output_reg: Register::new(load_cell_info.instr_offset),
						op: BinaryOperation::Sub,
					},
					BrainInstructionType::StoreValueIntoCell {
						value: RegOrImm::Reg(Register::new(load_cell_info.instr_offset)),
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
				BrainInstructionType::StoreValueIntoCell {
					value: RegOrImm::Imm(Immediate::cell(value.convert::<u64>())),
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
					imm: Immediate::pointer(offset.unsigned_abs().convert::<u64>()),
					output_reg: Register::new(1),
				},
				BrainInstructionType::PerformBinaryValueOperation {
					lhs: RegOrImm::Reg(Register::new(0)),
					rhs: RegOrImm::Imm(Immediate::pointer(offset.unsigned_abs().convert::<u64>())),
					output_reg: Register::new(1),
					op: if offset.is_positive() {
						BinaryOperation::Add
					} else {
						BinaryOperation::Sub
					},
				},
				BrainInstructionType::PerformBinaryValueOperation {
					lhs: RegOrImm::Reg(Register::new(1)),
					rhs: RegOrImm::Imm(Immediate::TAPE_SIZE_MINUS_ONE),
					output_reg: Register::new(2),
					op: BinaryOperation::BitwiseAnd,
				},
				BrainInstructionType::CalculateTapeOffset {
					tape_pointer_reg: Register::new(2),
					output_reg: Register::new(3),
				},
				BrainInstructionType::StoreValueIntoCell {
					value: RegOrImm::Imm(Immediate::cell(value.convert::<u64>())),
					pointer_reg: Register::new(3),
				},
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::MovePointer(offset) => [
				BrainInstructionType::LoadTapePointerIntoRegister {
					output_reg: Register::new(0),
				},
				BrainInstructionType::PerformBinaryValueOperation {
					lhs: RegOrImm::Reg(Register::new(0)),
					rhs: RegOrImm::Imm(Immediate::pointer(offset.unsigned_abs().convert::<u64>())),
					output_reg: Register::new(1),
					op: if offset.is_positive() {
						BinaryOperation::Add
					} else {
						BinaryOperation::Sub
					},
				},
				BrainInstructionType::PerformBinaryValueOperation {
					lhs: RegOrImm::Reg(Register::new(1)),
					rhs: RegOrImm::Imm(Immediate::TAPE_SIZE_MINUS_ONE),
					output_reg: Register::new(2),
					op: BinaryOperation::BitwiseAnd,
				},
				BrainInstructionType::StoreRegisterIntoTapePointer {
					input_reg: Register::new(2),
				},
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::MoveCellValue(CellOffsetOptions { value, offset }) => {
				let (current_cell_info, mut instrs) = LoadCellInformation::create(0, 0, None);

				instrs.extend([
					BrainInstructionType::StoreValueIntoCell {
						value: RegOrImm::Imm(Immediate::CELL_ZERO),
						pointer_reg: current_cell_info.pointer_reg,
					},
					BrainInstructionType::PerformBinaryValueOperation {
						lhs: RegOrImm::Reg(current_cell_info.cell_reg),
						rhs: RegOrImm::Imm(Immediate::cell(value.convert::<u64>())),
						output_reg: Register::new(current_cell_info.instr_offset + 1),
						op: BinaryOperation::Mul,
					},
				]);

				let (other_cell_info, mut other_cell_instrs) = LoadCellInformation::create(
					offset,
					current_cell_info.instr_offset + 2,
					Some(current_cell_info.tape_pointer_reg),
				);

				instrs.append(&mut other_cell_instrs);

				instrs.extend([
					BrainInstructionType::PerformBinaryValueOperation {
						lhs: RegOrImm::Reg(Register::new(current_cell_info.instr_offset + 1)),
						rhs: RegOrImm::Reg(other_cell_info.cell_reg),
						output_reg: Register::new(other_cell_info.instr_offset),
						op: BinaryOperation::Add,
					},
					BrainInstructionType::StoreValueIntoCell {
						value: RegOrImm::Reg(Register::new(other_cell_info.instr_offset)),
						pointer_reg: other_cell_info.pointer_reg,
					},
				]);

				instrs
					.into_iter()
					.map(|i| BrainInstruction::new(i, self.span().start))
					.collect()
			}
			&BrainOperationType::TakeCellValue(CellOffsetOptions { value, offset }) => {
				let (current_cell_info, mut instrs) = LoadCellInformation::create(0, 0, None);

				instrs.extend([
					BrainInstructionType::StoreValueIntoCell {
						value: RegOrImm::Imm(Immediate::CELL_ZERO),
						pointer_reg: current_cell_info.pointer_reg,
					},
					BrainInstructionType::PerformBinaryValueOperation {
						lhs: RegOrImm::Reg(current_cell_info.cell_reg),
						rhs: RegOrImm::Imm(Immediate::cell(value.convert::<u64>())),
						output_reg: Register::new(current_cell_info.instr_offset + 1),
						op: BinaryOperation::Mul,
					},
				]);

				let (other_cell_info, mut other_cell_instrs) = LoadCellInformation::create(
					offset,
					current_cell_info.instr_offset + 2,
					Some(current_cell_info.tape_pointer_reg),
				);

				instrs.append(&mut other_cell_instrs);

				instrs.extend([
					BrainInstructionType::PerformBinaryValueOperation {
						lhs: RegOrImm::Reg(Register::new(current_cell_info.instr_offset + 1)),
						rhs: RegOrImm::Reg(other_cell_info.cell_reg),
						output_reg: Register::new(other_cell_info.instr_offset),
						op: BinaryOperation::Add,
					},
					BrainInstructionType::StoreValueIntoCell {
						value: RegOrImm::Reg(Register::new(other_cell_info.instr_offset)),
						pointer_reg: other_cell_info.pointer_reg,
					},
					BrainInstructionType::StoreRegisterIntoTapePointer {
						input_reg: other_cell_info.tape_pointer_reg,
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
				BrainInstructionType::StoreValueIntoCell {
					value: RegOrImm::Reg(Register::new(0)),
					pointer_reg: Register::new(2),
				},
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::OutputCell(CellOffsetOptions { value, offset }) => {
				let (load_cell_info, mut instrs) = LoadCellInformation::create(offset, 0, None);

				if matches!(value, 0) {
					instrs.push(BrainInstructionType::OutputFromRegister {
						input_reg: load_cell_info.cell_reg,
					});
				} else {
					instrs.extend([
						BrainInstructionType::PerformBinaryValueOperation {
							lhs: RegOrImm::Reg(load_cell_info.cell_reg),
							rhs: RegOrImm::Imm(Immediate::cell(value.convert::<u64>())),
							output_reg: Register::new(load_cell_info.instr_offset),
							op: BinaryOperation::Add,
						},
						BrainInstructionType::OutputFromRegister {
							input_reg: Register::new(load_cell_info.instr_offset),
						},
					]);
				}

				instrs
					.into_iter()
					.map(|i| BrainInstruction::new(i, self.span().start))
					.collect()
			}
			&BrainOperationType::OutputValue(value) => [
				BrainInstructionType::StoreImmediateIntoRegister {
					imm: Immediate::cell(value.convert::<u64>()),
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
					BrainInstructionType::CompareValues {
						lhs: RegOrImm::Reg(Register::new(2)),
						rhs: RegOrImm::Imm(Immediate::CELL_ZERO),
						output_reg: Register::new(3),
					},
					BrainInstructionType::JumpIf {
						input_reg: Register::new(3),
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
