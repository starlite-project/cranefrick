use frick_instructions::{BrainInstruction, BrainInstructionType};
use frick_types::Register;

use crate::instrs::inner::Pass;

pub struct StoreLoadsPass;

impl Pass for StoreLoadsPass {
	fn run(&mut self, instrs: &mut Vec<frick_instructions::BrainInstruction>) -> bool {
		let mut changed_any = false;

		let mut last_instr = None;

		for instr in instrs {
			let Some(last) = last_instr else {
				last_instr = Some(instr.instr());
				continue;
			};

			if let (
				BrainInstructionType::StoreRegisterIntoTapePointer { input_reg },
				BrainInstructionType::LoadTapePointerIntoRegister { output_reg },
			) = (last, instr.instr())
			{
				*instr = BrainInstruction::new(
					BrainInstructionType::DuplicateRegister {
						input_reg: Register::new(input_reg.index()),
						output_reg: Register::new(output_reg.index()),
					},
					instr.byte_offset(),
				);
				changed_any = true;
			}

			last_instr = Some(instr.instr());
		}

		changed_any
	}
}
