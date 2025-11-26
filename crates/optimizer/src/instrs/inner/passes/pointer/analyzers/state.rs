use std::{
	collections::HashMap,
	ops::{Deref, DerefMut},
};

use frick_instructions::BrainInstructionType;

use crate::instrs::inner::Analyzer;

#[derive(Debug, Default, Clone)]
#[repr(transparent)]
pub struct PointerStateAnalyzer {
	states: HashMap<usize, PointerState>,
}

impl PointerStateAnalyzer {
	pub fn pointer_state_at(&self, index: usize) -> PointerState {
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

impl Analyzer for PointerStateAnalyzer {
	fn run(&mut self, instrs: &[frick_instructions::BrainInstruction]) -> bool {
		for (i, instr) in instrs.iter().copied().enumerate() {
			match instr.instr() {
				BrainInstructionType::LoadTapePointerIntoRegister { output_reg } => {
					self.insert(i, PointerState::new(Some(output_reg.index()), false));
				}
				BrainInstructionType::PerformBinaryRegisterOperation {
					lhs_reg, rhs_reg, ..
				} => {
					let previous_state = self.pointer_state_at(i - 1);

					if previous_state
						.register_index()
						.is_some_and(|x| x == lhs_reg.index() || x == rhs_reg.index())
					{
						self.insert(i, PointerState::new(previous_state.register_index(), true));
					}
				}
				BrainInstructionType::StoreRegisterIntoTapePointer { .. }
				| BrainInstructionType::StartLoop
				| BrainInstructionType::EndLoop => {
					self.insert(i, PointerState::default());
				}
				BrainInstructionType::CalculateTapeOffset { .. } => {}
				instr_ty if !matches!(i, 0) => {
					let prev_state = self.pointer_state_at(i - 1);

					if prev_state
						.register_index()
						.is_some_and(|reg| instr_ty.uses_register(reg))
					{
						self.insert(i, PointerState::default());
					}
				}
				_ => {}
			}
		}

		!matches!(
			(self.len(), self.values().next()),
			(
				1,
				Some(PointerState {
					register_index: None,
					dirty: false
				})
			)
		)
	}

	fn reset(&mut self) {
		self.insert(0, PointerState::default());
	}
}

impl Deref for PointerStateAnalyzer {
	type Target = HashMap<usize, PointerState>;

	fn deref(&self) -> &Self::Target {
		&self.states
	}
}

impl DerefMut for PointerStateAnalyzer {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.states
	}
}

#[derive(Debug, Clone, Copy)]
pub struct PointerState {
	register_index: Option<usize>,
	dirty: bool,
}

impl PointerState {
	const fn new(register_index: Option<usize>, dirty: bool) -> Self {
		Self {
			register_index,
			dirty,
		}
	}

	#[must_use]
	pub const fn register_index(self) -> Option<usize> {
		self.register_index
	}

	#[must_use]
	pub const fn is_dirty(self) -> bool {
		self.dirty
	}

	#[must_use]
	pub const fn is_value_known(self) -> bool {
		self.register_index.is_some()
	}
}

impl Default for PointerState {
	fn default() -> Self {
		Self::new(None, false)
	}
}
