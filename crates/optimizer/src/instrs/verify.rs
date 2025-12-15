use frick_instructions::{BrainInstruction, BrainInstructionType};
use frick_types::RegisterTypeEnum;
use rustc_hash::FxHashMap;

use super::InstructionsOptimizerError;

pub struct Verifier;

impl Verifier {
	pub fn run(instrs: &[BrainInstruction]) -> Result<(), InstructionsOptimizerError> {
		if !Self::check_loops(instrs) {
			return Err(InstructionsOptimizerError::LoopsNotValid);
		}

		Self::check_register_values(instrs)
	}

	fn check_loops(instrs: &[BrainInstruction]) -> bool {
		let mut loop_count = 0;

		for i in instrs {
			match i.instr() {
				BrainInstructionType::StartLoop => loop_count += 1,
				BrainInstructionType::EndLoop => loop_count -= 1,
				_ => {}
			}
		}

		matches!(loop_count, 0)
	}

	fn check_register_values(
		instrs: &[BrainInstruction],
	) -> Result<(), InstructionsOptimizerError> {
		let mut registers = FxHashMap::default();

		for i in instrs.iter().copied() {
			match i.instr() {
				BrainInstructionType::LoadCellIntoRegister {
					pointer_reg,
					output_reg,
				} => match registers.get(&pointer_reg.index()).copied() {
					Some(RegisterTypeEnum::Pointer) => {
						registers.insert(output_reg.index(), RegisterTypeEnum::Int);
					}
					found => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: pointer_reg.index(),
							expected: RegisterTypeEnum::Pointer,
							found,
						});
					}
				},
				BrainInstructionType::StoreRegisterIntoCell {
					value_reg,
					pointer_reg,
				} => {
					match (
						registers.get(&value_reg.index()).copied(),
						registers.get(&pointer_reg.index()).copied(),
					) {
						(Some(RegisterTypeEnum::Int), Some(RegisterTypeEnum::Pointer)) => {}
						(Some(RegisterTypeEnum::Int), found) => {
							return Err(InstructionsOptimizerError::RegisterInvalid {
								register: pointer_reg.index(),
								expected: RegisterTypeEnum::Pointer,
								found,
							});
						}
						(found, ..) => {
							return Err(InstructionsOptimizerError::RegisterInvalid {
								register: value_reg.index(),
								expected: RegisterTypeEnum::Int,
								found,
							});
						}
					}
				}
				BrainInstructionType::StoreImmediateIntoRegister { output_reg, .. }
				| BrainInstructionType::LoadTapePointerIntoRegister { output_reg }
				| BrainInstructionType::InputIntoRegister { output_reg } => {
					registers.insert(output_reg.index(), RegisterTypeEnum::Int);
				}
				BrainInstructionType::StoreRegisterIntoTapePointer { input_reg }
				| BrainInstructionType::OutputFromRegister { input_reg } => {
					match registers.get(&input_reg.index()).copied() {
						Some(RegisterTypeEnum::Int) => {}
						found => {
							return Err(InstructionsOptimizerError::RegisterInvalid {
								register: input_reg.index(),
								expected: RegisterTypeEnum::Int,
								found,
							});
						}
					}
				}
				BrainInstructionType::CalculateTapeOffset {
					tape_pointer_reg,
					output_reg,
				} => match registers.get(&tape_pointer_reg.index()).copied() {
					Some(RegisterTypeEnum::Int) => {
						registers.insert(output_reg.index(), RegisterTypeEnum::Pointer);
					}
					found => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: output_reg.index(),
							expected: RegisterTypeEnum::Int,
							found,
						});
					}
				},
				BrainInstructionType::PerformBinaryRegisterOperation {
					lhs_reg,
					rhs_reg,
					output_reg,
					..
				} => match (
					registers.get(&lhs_reg.index()).copied(),
					registers.get(&rhs_reg.index()).copied(),
				) {
					(Some(RegisterTypeEnum::Int), Some(RegisterTypeEnum::Int)) => {
						registers.insert(output_reg.index(), RegisterTypeEnum::Int);
					}
					(Some(RegisterTypeEnum::Int), found) => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: rhs_reg.index(),
							expected: RegisterTypeEnum::Int,
							found,
						});
					}
					(found, ..) => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: lhs_reg.index(),
							expected: RegisterTypeEnum::Int,
							found,
						});
					}
				},
				BrainInstructionType::DuplicateRegister {
					input_reg,
					output_reg,
				} => match registers.get(&input_reg.index()).copied() {
					Some(ty) => {
						registers.insert(output_reg.index(), ty);
					}
					found @ None => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: input_reg.index(),
							expected: RegisterTypeEnum::Any,
							found,
						});
					}
				},
				BrainInstructionType::CompareRegisterToRegister {
					lhs_reg,
					rhs_reg,
					output_reg,
				} => match (
					registers.get(&lhs_reg.index()).copied(),
					registers.get(&rhs_reg.index()).copied(),
				) {
					(Some(RegisterTypeEnum::Int), Some(RegisterTypeEnum::Int)) => {
						registers.insert(output_reg.index(), RegisterTypeEnum::Bool);
					}
					(Some(RegisterTypeEnum::Int), found) => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: rhs_reg.index(),
							expected: RegisterTypeEnum::Int,
							found,
						});
					}
					(found, ..) => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: lhs_reg.index(),
							expected: RegisterTypeEnum::Int,
							found,
						});
					}
				},
				BrainInstructionType::JumpIf { input_reg } => {
					match registers.get(&input_reg.index()).copied() {
						Some(RegisterTypeEnum::Bool) => {}
						found => {
							return Err(InstructionsOptimizerError::RegisterInvalid {
								register: input_reg.index(),
								expected: RegisterTypeEnum::Bool,
								found,
							});
						}
					}
				}
				_ => {}
			}
		}

		Ok(())
	}
}
