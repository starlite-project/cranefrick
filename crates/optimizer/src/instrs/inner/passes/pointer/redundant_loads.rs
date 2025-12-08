use frick_instructions::{BrainInstruction, BrainInstructionType};

use crate::instrs::inner::{Analyzer as _, Pass, passes::PointerStateAnalyzer};

pub struct PointerRedundantLoadsPass;

impl Pass for PointerRedundantLoadsPass {
	fn run(&mut self, instrs: &mut Vec<BrainInstruction>) -> bool {
		let mut state_analyzer = PointerStateAnalyzer::default();

		if !state_analyzer.reset_and_run(instrs) {
			tracing::debug!("no pointer analysis available");
			return false;
		}

		let indices_to_remove = instrs
			.iter()
			.copied()
			.enumerate()
			.skip(1)
			.filter_map(|(i, instr)| {
				let BrainInstructionType::LoadTapePointerIntoRegister { output_reg } =
					instr.instr()
				else {
					return None;
				};

				let prev_state = state_analyzer.pointer_state_at(i - 1);

				if !prev_state.is_dirty()
					&& prev_state
						.register_index()
						.is_some_and(|reg| reg == output_reg)
				{
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
