use std::cell::Cell;

use frick_instructions::{BrainInstruction, BrainInstructionType, Imm};
use frick_types::BinaryOperation;
use frick_utils::{Convert as _, SliceExt as _};

use crate::instrs::inner::Pass;

pub struct SimplifyMultiplicationPass;

impl SimplifyMultiplicationPass {
	fn transform_mul_to_shl(instrs: &mut [BrainInstruction]) -> bool {
		let mut changed_any = false;

		// Found this trick at https://internals.rust-lang.org/t/a-windows-mut-method-on-slice/16941/9
		let cell_of_slice = {
			let cell: &Cell<[BrainInstruction]> = Cell::from_mut(instrs);

			cell.as_slice_of_cells()
		};

		for x in cell_of_slice.windows_n::<2>() {
			match [x[0].get().instr(), x[1].get().instr()] {
				[
					BrainInstructionType::StoreImmediateIntoRegister { imm, output_reg },
					BrainInstructionType::PerformBinaryRegisterOperation {
						lhs_reg,
						rhs_reg,
						output_reg: binary_output_reg,
						op: BinaryOperation::Mul,
					},
				] if lhs_reg == output_reg
					|| rhs_reg == output_reg && imm.value().is_power_of_two() =>
				{
					let new_value = Imm::cell(imm.value().ilog2().convert::<u64>());

					if matches!(new_value.value(), 0) {
						continue;
					}

					x[0].set(BrainInstruction::new(
						BrainInstructionType::StoreImmediateIntoRegister {
							imm: new_value,
							output_reg,
						},
						x[0].get().byte_offset(),
					));
					x[1].set(BrainInstruction::new(
						BrainInstructionType::PerformBinaryRegisterOperation {
							lhs_reg,
							rhs_reg,
							output_reg: binary_output_reg,
							op: BinaryOperation::BitwiseShl,
						},
						x[1].get().byte_offset(),
					));

					changed_any = true;
				}
				_ => {}
			}
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

		for range in indices_to_replace {
			let Some(sliced_instrs) = instrs.get_mut(range) else {
				continue;
			};
		}

		removed_any
	}
}

impl Pass for SimplifyMultiplicationPass {
	fn run(&mut self, instrs: &mut Vec<BrainInstruction>) -> bool {
		Self::remove_redundant_multiplications(instrs) || Self::transform_mul_to_shl(instrs)
	}
}
