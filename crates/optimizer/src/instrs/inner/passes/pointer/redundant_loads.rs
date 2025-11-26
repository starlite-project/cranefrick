use frick_instructions::{BrainInstruction, BrainInstructionType};

use crate::instrs::inner::{Analyzer as _, Pass, passes::PointerStateAnalyzer};

#[derive(Debug, Default, Clone)]
#[repr(transparent)]
pub struct PointerRedundantLoadsPass {
	state_analyzer: PointerStateAnalyzer,
}

impl Pass for PointerRedundantLoadsPass {
	fn run(&mut self, instrs: &mut Vec<BrainInstruction>) -> bool {
		if !self.state_analyzer.reset_and_run(instrs) {
			tracing::debug!("no pointer analysis available");
			return false;
		}

		let indices_to_remove = instrs
			.iter()
			.copied()
			.enumerate()
			.skip(1)
			.filter_map(|(i, instr)| {
				if !matches!(
					instr.instr(),
					BrainInstructionType::LoadTapePointerIntoRegister { .. }
				) {
					return None;
				}

				let prev_state = self.state_analyzer.pointer_state_at(i - 1);

				if prev_state.is_value_known() && !prev_state.is_dirty() {
					Some(i)
				} else {
					None
				}
			})
			.collect::<Vec<_>>();

		let mut removed_any = false;
		for i in indices_to_remove.into_iter().rev() {
			instrs.remove(i);
			removed_any = true;
		}

		removed_any
	}
}
