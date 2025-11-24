use frick_instructions::{BrainInstruction, BrainInstructionType};

use super::{Pass, PointerAnalysis};

#[derive(Debug, Default, Clone)]
#[repr(transparent)]
pub struct PointerDCEPass {
	analysis: PointerAnalysis,
}

impl Pass for PointerDCEPass {
	fn run(&mut self, instrs: &mut Vec<BrainInstruction>) -> bool {
		if !self.analysis.analyze(instrs) {
			tracing::debug!("no pointer analysis available");
			return false;
		}

		let mut indices_to_remove = Vec::<usize>::with_capacity(self.analysis.len());

		for (i, instr) in instrs.iter().copied().enumerate().skip(1) {
			if !matches!(instr.instr(), BrainInstructionType::LoadPointer) {
				continue;
			}

			let prev_state = self.analysis.pointer_state_at(i - 1);

			if !prev_state.value_known {
				continue;
			}

			indices_to_remove.push(i);
		}

		for i in indices_to_remove.iter().copied().rev() {
			instrs.remove(i);
		}

		!indices_to_remove.is_empty()
	}
}
