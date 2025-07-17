mod opt;

use alloc::vec::Vec;
use core::{
	ops::{Deref, DerefMut},
	slice,
};

use cranefrick_hlir::BrainHlir;
use serde::{Deserialize, Serialize};
use tracing::info;

use self::opt::{passes, passes::remove_early_loops, run_loop_pass, run_peephole_pass};
use super::BrainMlir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Compiler {
	inner: Vec<BrainMlir>,
}

impl Compiler {
	#[must_use]
	pub const fn new() -> Self {
		Self { inner: Vec::new() }
	}

	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			inner: Vec::with_capacity(capacity),
		}
	}

	pub fn push(&mut self, i: BrainMlir) {
		self.inner.push(i);
	}

	#[tracing::instrument("optimize mlir", skip(self))]
	pub fn optimize(&mut self) {
		let mut iteration = 0usize;

		let mut progress = self.optimization_pass(iteration);

		while progress {
			iteration += 1;
			progress = self.optimization_pass(iteration);
		}
	}

	#[tracing::instrument("run passes", skip(self))]
	fn optimization_pass(&mut self, iteration: usize) -> bool {
		let starting_instruction_count = self.inner.len();
		let mut progress = false;

		self.run_all_passes(&mut progress);

		info!("{starting_instruction_count} -> {}", self.inner.len());

		progress
	}

	fn run_all_passes(&mut self, progress: &mut bool) {
		self.pass_info("combine instructions");
		*progress |= run_peephole_pass(&mut *self, passes::combine_instructions);

		self.pass_info("optimize clear cell instructions");
		*progress |= run_peephole_pass(&mut *self, passes::clear_cell);

		self.pass_info("remove unreachable loops");
		*progress |= run_peephole_pass(&mut *self, passes::remove_unreachable_loops);

		self.pass_info("remove infinite loops");
		*progress |= run_loop_pass(&mut *self, passes::remove_infinite_loops);

		self.pass_info("remove empty loops");
		*progress |= run_loop_pass(&mut *self, passes::remove_empty_loops);

		self.pass_info("remove useless beginning loops");
		*progress |= remove_early_loops(&mut *self);
	}

	fn pass_info(&self, pass: &str) {
		let op_count = self.inner.len();
		info!("running pass {pass} with {op_count} instructions");
	}
}

impl Default for Compiler {
	fn default() -> Self {
		Self::new()
	}
}

impl Deref for Compiler {
	type Target = Vec<BrainMlir>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl DerefMut for Compiler {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}

impl Extend<BrainMlir> for Compiler {
	fn extend<T>(&mut self, iter: T)
	where
		T: IntoIterator<Item = BrainMlir>,
	{
		self.inner.extend(iter);
	}
}

impl Extend<BrainHlir> for Compiler {
	fn extend<T>(&mut self, iter: T)
	where
		T: IntoIterator<Item = BrainHlir>,
	{
		let old = iter.into_iter().collect::<Vec<_>>();

		self.extend(fix_loops(&old));
	}
}

impl FromIterator<BrainMlir> for Compiler {
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = BrainMlir>,
	{
		Self {
			inner: Vec::from_iter(iter),
		}
	}
}

impl FromIterator<BrainHlir> for Compiler {
	fn from_iter<T: IntoIterator<Item = BrainHlir>>(iter: T) -> Self {
		let old = iter.into_iter().collect::<Vec<_>>();

		Self {
			inner: fix_loops(&old),
		}
	}
}

impl<'a> IntoIterator for &'a Compiler {
	type IntoIter = slice::Iter<'a, BrainMlir>;
	type Item = &'a BrainMlir;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.iter()
	}
}

impl<'a> IntoIterator for &'a mut Compiler {
	type IntoIter = slice::IterMut<'a, BrainMlir>;
	type Item = &'a mut BrainMlir;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.iter_mut()
	}
}

impl IntoIterator for Compiler {
	type IntoIter = alloc::vec::IntoIter<BrainMlir>;
	type Item = BrainMlir;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.into_iter()
	}
}

fn fix_loops(program: &[BrainHlir]) -> Vec<BrainMlir> {
	let mut current_stack = Vec::new();
	let mut loop_stack = 0usize;
	let mut loop_start = 0usize;

	for (i, op) in program.iter().enumerate() {
		if matches!(loop_stack, 0) {
			if let Some(instr) = match op {
				BrainHlir::EndLoop => unreachable!(),
				BrainHlir::StartLoop => {
					loop_start = i;
					loop_stack += 1;
					None
				}
				// i => Some(i.clone()),
				BrainHlir::IncrementCell => Some(BrainMlir::change_cell(1)),
				BrainHlir::DecrementCell => Some(BrainMlir::change_cell(-1)),
				BrainHlir::MovePtrLeft => Some(BrainMlir::move_ptr(-1)),
				BrainHlir::MovePtrRight => Some(BrainMlir::move_ptr(1)),
				BrainHlir::GetInput => Some(BrainMlir::get_input()),
				BrainHlir::PutOutput => Some(BrainMlir::put_output()),
			} {
				current_stack.push(instr);
			}
		} else {
			match op {
				BrainHlir::StartLoop => loop_stack += 1,
				BrainHlir::EndLoop => {
					loop_stack -= 1;
					if matches!(loop_stack, 0) {
						current_stack.push(BrainMlir::dynamic_loop({
							let s = &program[loop_start + 1..i];
							fix_loops(s)
						}));
					}
				}
				_ => {}
			}
		}
	}

	current_stack
}
