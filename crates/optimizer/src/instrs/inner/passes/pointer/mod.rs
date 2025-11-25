mod redundant_loads;

use std::{
	collections::BTreeMap,
	ops::{Deref, DerefMut},
};

use frick_instructions::{BrainInstruction, BrainInstructionType};

pub use self::redundant_loads::*;
use crate::instrs::inner::{Analyzer, Pass};

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
}

impl Analyzer for PointerAnalysis {
	fn run(&mut self, instrs: &[BrainInstruction]) -> bool {
		self.insert(0, PointerState::default());

		for (i, instr) in instrs.iter().enumerate() {
			match instr.instr() {
				BrainInstructionType::LoadPointer => {
					self.insert(i, PointerState::new(false, true));
				}
				BrainInstructionType::OffsetPointer { .. } => {
					self.insert(i, PointerState::new(true, false));
				}
				BrainInstructionType::StorePointer
				| BrainInstructionType::StartLoop
				| BrainInstructionType::EndLoop => {
					self.insert(i, PointerState::default());
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

#[derive(Debug, Clone, Copy)]
struct PointerState {
	dirty: bool,
	value_known: bool,
}

impl PointerState {
	const fn new(dirty: bool, value_known: bool) -> Self {
		Self { dirty, value_known }
	}

	const fn is_dirty(self) -> bool {
		self.dirty
	}

	const fn is_value_known(self) -> bool {
		self.value_known
	}
}

impl Default for PointerState {
	fn default() -> Self {
		Self::new(false, false)
	}
}
