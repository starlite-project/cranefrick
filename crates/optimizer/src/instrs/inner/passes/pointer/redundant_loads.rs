use frick_instructions::{BrainInstruction, BrainInstructionType};

use super::{Analyzer as _, Pass, PointerAnalysis};

#[derive(Debug, Default, Clone)]
#[repr(transparent)]
pub struct PointerRedundantLoadsPass {
	analysis: PointerAnalysis,
}

impl Pass for PointerRedundantLoadsPass {
	fn run(&mut self, instrs: &mut Vec<BrainInstruction>) -> bool {
		if !self.analysis.run(instrs) {
			tracing::debug!("no pointer analysis available");
			return false;
		}

		let indices_to_remove = instrs
			.iter()
			.copied()
			.enumerate()
			.skip(1)
			.filter_map(|(i, instr)| {
				if !matches!(instr.instr(), BrainInstructionType::LoadPointer) {
					return None;
				}

				let prev_state = self.analysis.pointer_state_at(i - 1);

				if prev_state.is_value_known() {
					Some(i)
				} else {
					None
				}
			})
			.collect::<Vec<_>>();

		let mut removed_any = false;
		for i in indices_to_remove.iter().copied().rev() {
			instrs.remove(i);
			removed_any = true;
		}

		removed_any
	}
}
