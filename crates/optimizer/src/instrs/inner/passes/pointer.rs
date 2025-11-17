use std::collections::{BTreeMap, HashMap};

use frick_instructions::{BrainInstruction, BrainInstructionType};

use super::{AnalysisPass, Pass};

#[derive(Debug, Default, Clone)]
pub struct PointerDCEPass {
	analysis: PointerAnalysisPass,
}

impl Pass for PointerDCEPass {
	fn run(&mut self, instrs: &mut Vec<BrainInstruction>) -> bool {
		for (i, instr) in instrs.iter().enumerate() {
			let state = self.analysis.pointer_state_at(i);

			tracing::info!(?i, ?state, ?instr);
		}

		false
	}

	fn run_analysis_passes(&mut self, instrs: &[BrainInstruction]) -> bool {
		self.analysis.run(instrs)
	}
}

#[derive(Debug, Default, Clone)]
pub struct PointerAnalysisPass {
	states: BTreeMap<usize, PointerState>,
}

impl PointerAnalysisPass {
	pub fn pointer_state_at(&self, index: usize) -> PointerState {
		if let Some(state) = self.states.get(&index).copied() {
			return state;
		}

		if index > 0 {
			self.pointer_state_at(index - 1)
		} else {
			unreachable!()
		}
	}
}

impl AnalysisPass for PointerAnalysisPass {
	fn run(&mut self, instrs: &[BrainInstruction]) -> bool {
		self.states.insert(
			0,
			PointerState {
				dirty: false,
				value_known: false,
			},
		);

		for (i, instr) in instrs.iter().enumerate() {
			match instr.instr() {
				BrainInstructionType::LoadPointer => {
					self.states.insert(
						i,
						PointerState {
							dirty: false,
							value_known: true,
						},
					);
				}
				BrainInstructionType::OffsetPointer { .. } => {
					self.states.insert(
						i,
						PointerState {
							dirty: true,
							value_known: false,
						},
					);
				}
				BrainInstructionType::StorePointer => {
					self.states.insert(
						i,
						PointerState {
							dirty: false,
							value_known: false,
						},
					);
				}
				_ => {}
			}
		}

		!self.states.is_empty()
	}
}

#[derive(Debug, Clone, Copy)]
pub struct PointerState {
	dirty: bool,
	value_known: bool,
}
