use alloc::vec::Vec;

use frick_types::{BinaryOperation, Immediate, Int, Pointer, RegOrImm, Register};
use frick_utils::Convert as _;

use super::BrainInstructionType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadCellInformation {
	pub cell_reg: Register<Int>,
	pub tape_pointer_reg: Register<Int>,
	pub pointer_reg: Register<Pointer>,
	pub instr_offset: usize,
}

impl LoadCellInformation {
	pub fn create(
		offset: i32,
		instr_offset: usize,
		tape_pointer_reg: Option<Register<Int>>,
	) -> (Self, Vec<BrainInstructionType>) {
		let mut instrs = Vec::new();

		let (cell_reg, tape_pointer_reg, pointer_reg, instr_offset) =
			match (offset, tape_pointer_reg) {
				(0, Some(tape_pointer_reg)) => {
					let pointer_reg = Register::new(instr_offset);
					let cell_reg = Register::new(instr_offset + 1);

					instrs.extend([
						BrainInstructionType::CalculateTapeOffset {
							tape_pointer_reg,
							output_reg: pointer_reg,
						},
						BrainInstructionType::LoadCellIntoRegister {
							pointer_reg,
							output_reg: cell_reg,
						},
					]);

					(cell_reg, tape_pointer_reg, pointer_reg, instr_offset + 2)
				}
				(0, None) => {
					let tape_pointer_reg = Register::new(instr_offset);
					let pointer_reg = Register::new(instr_offset + 1);
					let cell_reg = Register::new(instr_offset + 2);

					instrs.extend([
						BrainInstructionType::LoadTapePointerIntoRegister {
							output_reg: tape_pointer_reg,
						},
						BrainInstructionType::CalculateTapeOffset {
							tape_pointer_reg,
							output_reg: pointer_reg,
						},
						BrainInstructionType::LoadCellIntoRegister {
							pointer_reg,
							output_reg: cell_reg,
						},
					]);

					(cell_reg, tape_pointer_reg, pointer_reg, instr_offset + 3)
				}
				(offset, Some(tape_pointer_reg)) => {
					let new_tape_pointer_reg = Register::new(instr_offset + 3);
					let pointer_reg = Register::new(instr_offset + 4);
					let cell_reg = Register::new(instr_offset + 5);
					instrs.extend([
						BrainInstructionType::PerformBinaryValueOperation {
							lhs: RegOrImm::Reg(tape_pointer_reg),
							rhs: RegOrImm::Imm(Immediate::pointer(
								offset.unsigned_abs().convert::<u64>(),
							)),
							output_reg: Register::new(instr_offset),
							op: if offset.is_positive() {
								BinaryOperation::Add
							} else {
								BinaryOperation::Sub
							},
						},
						BrainInstructionType::PerformBinaryValueOperation {
							lhs: RegOrImm::Reg(Register::new(instr_offset)),
							rhs: RegOrImm::Imm(Immediate::TAPE_SIZE_MINUS_ONE),
							output_reg: new_tape_pointer_reg,
							op: BinaryOperation::BitwiseAnd,
						},
						BrainInstructionType::CalculateTapeOffset {
							tape_pointer_reg: new_tape_pointer_reg,
							output_reg: pointer_reg,
						},
						BrainInstructionType::LoadCellIntoRegister {
							pointer_reg,
							output_reg: cell_reg,
						},
					]);

					(
						cell_reg,
						new_tape_pointer_reg,
						pointer_reg,
						instr_offset + 6,
					)
				}
				(offset, None) => {
					let tape_pointer_reg = Register::new(instr_offset + 4);
					let pointer_reg = Register::new(instr_offset + 5);
					let cell_reg = Register::new(instr_offset + 6);

					instrs.extend([
						BrainInstructionType::LoadTapePointerIntoRegister {
							output_reg: Register::new(instr_offset),
						},
						BrainInstructionType::PerformBinaryValueOperation {
							lhs: RegOrImm::Reg(Register::new(instr_offset)),
							rhs: RegOrImm::Imm(Immediate::pointer(
								offset.unsigned_abs().convert::<u64>(),
							)),
							output_reg: Register::new(instr_offset + 2),
							op: if offset.is_positive() {
								BinaryOperation::Add
							} else {
								BinaryOperation::Sub
							},
						},
						BrainInstructionType::PerformBinaryValueOperation {
							lhs: RegOrImm::Reg(Register::new(instr_offset + 2)),
							rhs: RegOrImm::Imm(Immediate::TAPE_SIZE_MINUS_ONE),
							output_reg: tape_pointer_reg,
							op: BinaryOperation::BitwiseAnd,
						},
						BrainInstructionType::CalculateTapeOffset {
							tape_pointer_reg,
							output_reg: pointer_reg,
						},
						BrainInstructionType::LoadCellIntoRegister {
							pointer_reg,
							output_reg: cell_reg,
						},
					]);

					(cell_reg, tape_pointer_reg, pointer_reg, instr_offset + 7)
				}
			};

		(
			Self {
				cell_reg,
				tape_pointer_reg,
				pointer_reg,
				instr_offset,
			},
			instrs,
		)
	}
}
