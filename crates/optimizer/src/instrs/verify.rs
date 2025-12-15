use frick_instructions::{BrainInstruction, BrainInstructionType};
use frick_spec::POINTER_SIZE;
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
						registers.insert(output_reg.index(), RegisterTypeEnum::Int(Some(8)));
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
						(Some(RegisterTypeEnum::Int(Some(8))), Some(RegisterTypeEnum::Pointer)) => {
						}
						(Some(RegisterTypeEnum::Int(Some(8))), found) => {
							return Err(InstructionsOptimizerError::RegisterInvalid {
								register: pointer_reg.index(),
								expected: RegisterTypeEnum::Pointer,
								found,
							});
						}
						(found, ..) => {
							return Err(InstructionsOptimizerError::RegisterInvalid {
								register: value_reg.index(),
								expected: RegisterTypeEnum::Int(Some(8)),
								found,
							});
						}
					}
				}
				BrainInstructionType::StoreImmediateIntoRegister { imm, output_reg } => {
					registers.insert(
						output_reg.index(),
						RegisterTypeEnum::Int(Some(imm.size() as usize)),
					);
				}
				BrainInstructionType::LoadTapePointerIntoRegister { output_reg } => {
					registers.insert(
						output_reg.index(),
						RegisterTypeEnum::Int(Some(POINTER_SIZE)),
					);
				}
				BrainInstructionType::StoreRegisterIntoTapePointer { input_reg } => {
					match registers.get(&input_reg.index()).copied() {
						Some(RegisterTypeEnum::Int(Some(64))) => {}
						found => {
							return Err(InstructionsOptimizerError::RegisterInvalid {
								register: input_reg.index(),
								expected: RegisterTypeEnum::Int(Some(64)),
								found,
							});
						}
					}
				}
				BrainInstructionType::CalculateTapeOffset {
					tape_pointer_reg,
					output_reg,
				} => match registers.get(&tape_pointer_reg.index()).copied() {
					Some(RegisterTypeEnum::Int(Some(64))) => {
						registers.insert(output_reg.index(), RegisterTypeEnum::Pointer);
					}
					found => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: output_reg.index(),
							expected: RegisterTypeEnum::Int(Some(64)),
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
					(Some(RegisterTypeEnum::Int(a)), Some(RegisterTypeEnum::Int(b))) if a == b => {
						registers.insert(output_reg.index(), RegisterTypeEnum::Int(a));
					}
					(Some(RegisterTypeEnum::Int(a)), found) => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: rhs_reg.index(),
							expected: RegisterTypeEnum::Int(a),
							found,
						});
					}
					(found, ..) => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: lhs_reg.index(),
							expected: RegisterTypeEnum::Int(None),
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
				BrainInstructionType::InputIntoRegister { output_reg } => {
					registers.insert(output_reg.index(), RegisterTypeEnum::Int(Some(8)));
				}
				BrainInstructionType::OutputFromRegister { input_reg } => {
					match registers.get(&input_reg.index()).copied() {
						Some(RegisterTypeEnum::Int(Some(8))) => {}
						found => {
							return Err(InstructionsOptimizerError::RegisterInvalid {
								register: input_reg.index(),
								expected: RegisterTypeEnum::Int(Some(8)),
								found,
							});
						}
					}
				}
				BrainInstructionType::CompareRegisterToRegister {
					lhs_reg,
					rhs_reg,
					output_reg,
				} => match (
					registers.get(&lhs_reg.index()).copied(),
					registers.get(&rhs_reg.index()).copied(),
				) {
					(
						Some(RegisterTypeEnum::Int(Some(8))),
						Some(RegisterTypeEnum::Int(Some(8))),
					) => {
						registers.insert(output_reg.index(), RegisterTypeEnum::Bool);
					}
					(Some(RegisterTypeEnum::Int(Some(8))), found) => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: rhs_reg.index(),
							expected: RegisterTypeEnum::Int(Some(8)),
							found,
						});
					}
					(found, ..) => {
						return Err(InstructionsOptimizerError::RegisterInvalid {
							register: lhs_reg.index(),
							expected: RegisterTypeEnum::Int(Some(8)),
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
