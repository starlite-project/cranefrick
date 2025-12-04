use alloc::vec::Vec;

use frick_spec::TAPE_SIZE;
use frick_types::{BinaryOperation, Int, Pointer, Register};
use frick_utils::Convert as _;

use super::BrainInstructionType;
use crate::Imm;

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
						BrainInstructionType::StoreImmediateIntoRegister {
							imm: Imm::pointer(offset.unsigned_abs().convert::<u64>()),
							output_reg: Register::new(instr_offset),
						},
						BrainInstructionType::PerformBinaryRegisterOperation {
							lhs_reg: tape_pointer_reg,
							rhs_reg: Register::new(instr_offset),
							output_reg: Register::new(instr_offset + 1),
							op: if offset.is_positive() {
								BinaryOperation::Add
							} else {
								BinaryOperation::Sub
							},
						},
						BrainInstructionType::StoreImmediateIntoRegister {
							imm: Imm::pointer(TAPE_SIZE as u64 - 1),
							output_reg: Register::new(instr_offset + 2),
						},
						BrainInstructionType::PerformBinaryRegisterOperation {
							lhs_reg: Register::new(instr_offset + 1),
							rhs_reg: Register::new(instr_offset + 2),
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
						BrainInstructionType::StoreImmediateIntoRegister {
							imm: Imm::pointer(offset.unsigned_abs().convert::<u64>()),
							output_reg: Register::new(instr_offset + 1),
						},
						BrainInstructionType::PerformBinaryRegisterOperation {
							lhs_reg: Register::new(instr_offset),
							rhs_reg: Register::new(instr_offset + 1),
							output_reg: Register::new(instr_offset + 2),
							op: if offset.is_positive() {
								BinaryOperation::Add
							} else {
								BinaryOperation::Sub
							},
						},
						BrainInstructionType::StoreImmediateIntoRegister {
							imm: Imm::pointer(TAPE_SIZE as u64 - 1),
							output_reg: Register::new(instr_offset + 3),
						},
						BrainInstructionType::PerformBinaryRegisterOperation {
							lhs_reg: Register::new(instr_offset + 2),
							rhs_reg: Register::new(instr_offset + 3),
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
