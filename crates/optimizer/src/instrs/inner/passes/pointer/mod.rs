mod redundant_loads;

use std::{
	collections::BTreeMap,
	ops::{Deref, DerefMut},
};

use frick_instructions::{BrainInstruction, BrainInstructionType};

pub use self::redundant_loads::*;
use super::Pass;

#[derive(Debug, Default, Clone)]
#[repr(transparent)]
struct PointerAnalysis {
	states: BTreeMap<usize, PointerState>,
}

impl PointerAnalysis {
	fn pointer_state_at(&self, index: usize) -> PointerState {
		if let Some(state) = self.get(&index).copied() {
			return state;
		}

		if index > 0 {
			self.pointer_state_at(index - 1)
		} else {
			unreachable!()
		}
	}

	fn analyze(&mut self, instrs: &[BrainInstruction]) -> bool {
		self.insert(0, PointerState::default());

		for (i, instr) in instrs.iter().enumerate() {
			match instr.instr() {
				BrainInstructionType::LoadPointer => {
					self.insert(
						i,
						PointerState {
							dirty: false,
							value_known: true,
						},
					);
				}
				BrainInstructionType::OffsetPointer { .. } => {
					self.insert(
						i,
						PointerState {
							dirty: true,
							value_known: false,
						},
					);
				}
				BrainInstructionType::StorePointer
				| BrainInstructionType::JumpIf { .. }
				| BrainInstructionType::JumpToHeader
				| BrainInstructionType::StartLoop
				| BrainInstructionType::EndLoop => {
					self.insert(
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

		!matches!(
			(self.len(), self.values().next()),
			(
				1,
				Some(PointerState {
					dirty: false,
					value_known: false
				})
			)
		)
	}
}

impl Deref for PointerAnalysis {
	type Target = BTreeMap<usize, PointerState>;

	fn deref(&self) -> &Self::Target {
		&self.states
	}
}

impl DerefMut for PointerAnalysis {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.states
	}
}

#[derive(Debug, Default, Clone, Copy)]
struct PointerState {
	dirty: bool,
	value_known: bool,
}
