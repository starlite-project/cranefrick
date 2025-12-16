use frick_instructions::{BrainInstruction, BrainInstructionType, Imm};
use frick_types::BinaryOperation;
use frick_utils::Convert as _;

use crate::instrs::inner::Pass;

pub struct SimplifyMultiplicationPass;

impl SimplifyMultiplicationPass {
	fn transform_mul_to_shl(instrs: &mut Vec<BrainInstruction>) -> bool {
		let mut changed_any = false;

		let mut i = 1;

		while i < instrs.len() {
			if let [
				BrainInstructionType::StoreImmediateIntoRegister { imm, output_reg },
				BrainInstructionType::PerformBinaryRegisterOperation {
					lhs_reg,
					rhs_reg,
					output_reg: binary_output_reg,
					op: BinaryOperation::Mul,
				},
			] = [instrs[i - 1].instr(), instrs[i].instr()]
				&& (output_reg == lhs_reg || output_reg == rhs_reg)
				&& imm.value().is_power_of_two()
			{
				match imm.value() {
					2 => {
						let other_reg = if output_reg == lhs_reg {
							rhs_reg
						} else {
							lhs_reg
						};

						*instrs[i] = BrainInstructionType::PerformBinaryRegisterOperation {
							lhs_reg: other_reg,
							rhs_reg: other_reg,
							output_reg: binary_output_reg,
							op: BinaryOperation::Add,
						};

						instrs.remove(i - 1);

						i -= 1;
						changed_any = true;
					}
					x => {
						let new_imm = Imm::cell(x.ilog2().convert::<u64>());

						*instrs[i - 1] = BrainInstructionType::StoreImmediateIntoRegister {
							imm: new_imm,
							output_reg,
						};
						*instrs[i] = BrainInstructionType::PerformBinaryRegisterOperation {
							lhs_reg,
							rhs_reg,
							output_reg: binary_output_reg,
							op: BinaryOperation::BitwiseShl,
						};

						changed_any = true;
					}
				}
			}

			i += 1;
		}

		changed_any
	}

	fn remove_redundant_multiplications(instrs: &mut Vec<BrainInstruction>) -> bool {
		let mut removed_any = false;

		let mut i = 1;

		let mut indices_to_replace = Vec::new();

		while i < instrs.len() {
			if let [
				BrainInstructionType::StoreImmediateIntoRegister { imm, output_reg },
				BrainInstructionType::PerformBinaryRegisterOperation {
					lhs_reg,
					rhs_reg,
					op: BinaryOperation::Mul,
					..
				},
			] = [instrs[i - 1].instr(), instrs[i].instr()]
				&& (output_reg == lhs_reg || output_reg == rhs_reg)
				&& matches!(imm.value(), 1)
			{
				indices_to_replace.push((i - 1)..=i);
			}

			i += 1;
		}

		for range in indices_to_replace.into_iter().rev() {
			let last_instr_idx = *range.end();

			let Some(sliced_instrs) = instrs.get_mut(range) else {
				continue;
			};

			assert_eq!(sliced_instrs.len(), 2);

			let BrainInstructionType::StoreImmediateIntoRegister { output_reg, .. } =
				*sliced_instrs[0]
			else {
				unreachable!();
			};

			let BrainInstructionType::PerformBinaryRegisterOperation {
				lhs_reg,
				rhs_reg,
				output_reg: binary_output_reg,
				..
			} = *sliced_instrs[1]
			else {
				unreachable!()
			};

			let reg_with_value = if output_reg == lhs_reg {
				rhs_reg
			} else {
				lhs_reg
			};

			*sliced_instrs[0] = BrainInstructionType::DuplicateRegister {
				input_reg: reg_with_value.cast(),
				output_reg: binary_output_reg.cast(),
			};
			instrs.remove(last_instr_idx);

			removed_any = true;
		}

		removed_any
	}
}

impl Pass for SimplifyMultiplicationPass {
	fn run(&mut self, instrs: &mut Vec<BrainInstruction>) -> bool {
		Self::remove_redundant_multiplications(instrs) || Self::transform_mul_to_shl(instrs)
	}
}
