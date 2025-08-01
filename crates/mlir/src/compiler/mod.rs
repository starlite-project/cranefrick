mod opt;

use alloc::vec::Vec;
use core::{
	ops::{Deref, DerefMut},
	slice,
};

use cranefrick_hlir::BrainHlir;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use self::opt::{passes, run_loop_pass, run_peephole_pass};
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

		info!(iterations = iteration, "finished optimize mlir");
	}

	#[tracing::instrument("run passes", skip(self))]
	fn optimization_pass(&mut self, iteration: usize) -> bool {
		let mut progress = false;

		self.run_all_passes(&mut progress);

		progress
	}

	fn run_all_passes(&mut self, progress: &mut bool) {
		self.pass_info("combine relavent instructions");
		*progress |= run_peephole_pass(self, passes::optimize_consecutive_instructions);

		self.pass_info("adding relavent offsets");
		*progress |= run_peephole_pass(self, passes::add_offsets);

		self.pass_info("fixing beginning instructions");
		*progress |= passes::fix_beginning_instructions(self);

		self.pass_info("optimize clear-cell instructions");
		*progress |= run_loop_pass(self, passes::clear_cell);

		self.pass_info("optimize set-based instructions");
		*progress |= run_peephole_pass(self, passes::optimize_sets);

		self.pass_info("optimize find-zere instructions");
		*progress |= run_loop_pass(self, passes::optimize_find_zero);

		self.pass_info("removing no-op instructions");
		*progress |= run_peephole_pass(self, passes::remove_noop_instructions);

		self.pass_info("removing unreachable loops");
		*progress |= run_peephole_pass(self, passes::remove_unreachable_loops);

		self.pass_info("removing infinite loops");
		*progress |= run_loop_pass(self, passes::remove_infinite_loops);

		self.pass_info("removing empty loops");
		*progress |= run_loop_pass(self, passes::remove_empty_loops);

		self.pass_info("unrolling no-move dynamic loops");
		*progress |= run_peephole_pass(self, passes::unroll_basic_dynamic_loop);

		self.pass_info("partially unrolling no-move dynamic loops");
		*progress |= run_peephole_pass(self, passes::partially_unroll_basic_dynamic_loop);

		self.pass_info("sorting cell changes");
		*progress |= run_peephole_pass(self, passes::sort_changes);

		self.pass_info("optimize scale and shift value instructions");
		*progress |= run_loop_pass(self, passes::optimize_move_value);
		*progress |= run_peephole_pass(self, passes::optimize_take_value);
		*progress |= run_peephole_pass(self, passes::optimize_fetch_value);

		self.pass_info("optimize if nz");
		*progress |= run_loop_pass(self, passes::optimize_if_nz);

		self.pass_info("optimize write calls");
		*progress |= run_peephole_pass(self, passes::optimize_writes);

		self.pass_info("optimize no-op loop");
		*progress |= run_loop_pass(self, passes::unroll_noop_loop);

		self.pass_info("remove redundant take instructions");
		*progress |= run_peephole_pass(self, passes::remove_redundant_takes);

		self.pass_info("optimize constant shifts");
		*progress |= run_peephole_pass(self, passes::optimize_constant_shifts);
	}

	fn pass_info(&self, pass: &str) {
		let (op_count, dloop_count) = self.stats();
		debug!("running pass {pass} with {op_count} instructions ({dloop_count}) loops");
	}

	fn stats(&self) -> (usize, usize) {
		Self::stats_of(self)
	}

	#[expect(clippy::single_match)]
	fn stats_of(ops: &[BrainMlir]) -> (usize, usize) {
		let mut op_count = 0;
		let mut dloop_count = 0;

		for op in ops {
			op_count += 1;
			match op {
				BrainMlir::DynamicLoop(l) => {
					let (ops, dloops) = Self::stats_of(l);

					op_count += ops;
					dloop_count += 1;
					dloop_count += dloops;
				}
				_ => {}
			}
		}

		(op_count, dloop_count)
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
				BrainHlir::IncrementCell => Some(BrainMlir::change_cell(1)),
				BrainHlir::DecrementCell => Some(BrainMlir::change_cell(-1)),
				BrainHlir::MovePtrLeft => Some(BrainMlir::move_pointer(-1)),
				BrainHlir::MovePtrRight => Some(BrainMlir::move_pointer(1)),
				BrainHlir::GetInput => Some(BrainMlir::input_cell()),
				BrainHlir::PutOutput => Some(BrainMlir::output_current_cell()),
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
