mod opt;
mod parse;

use std::{
	ops::{Deref, DerefMut},
	slice,
};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use self::opt::{passes, run_loop_pass, run_peephole_pass};
pub use self::parse::*;
use super::BrainIr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Compiler {
	inner: Vec<BrainIr>,
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

	pub fn push(&mut self, i: BrainIr) {
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
		*progress |= run_peephole_pass(self, passes::optimize_set_range);

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

		self.pass_info("sorting cell changes");
		*progress |= run_peephole_pass(self, passes::sort_changes);

		self.pass_info("optimize scale and shift value instructions");
		*progress |= run_loop_pass(self, passes::optimize_move_value);
		*progress |= run_peephole_pass(self, passes::optimize_take_value);
		*progress |= run_peephole_pass(self, passes::optimize_fetch_value);
		*progress |= run_peephole_pass(self, passes::optimize_replace_value);
		*progress |= run_peephole_pass(self, passes::optimize_scale_value);

		self.pass_info("optimize write calls");
		*progress |= run_peephole_pass(self, passes::optimize_writes);
		*progress |= run_peephole_pass(self, passes::optimize_sets_and_writes);
		*progress |= run_peephole_pass(self, passes::optimize_offset_writes);

		self.pass_info("optimize no-op loop");
		*progress |= run_loop_pass(self, passes::unroll_noop_loop);

		self.pass_info("remove redundant take instructions");
		*progress |= run_peephole_pass(self, passes::remove_redundant_takes);

		self.pass_info("optimize constant shifts");
		*progress |= run_peephole_pass(self, passes::optimize_constant_shifts);

		self.pass_info("remove unnecessary offsets");
		*progress |= run_peephole_pass(self, passes::remove_offsets);

		self.pass_info("optimize sub cell");
		*progress |= run_loop_pass(self, passes::optimize_sub_cell);

		self.pass_info("optimizing if_nz");
		*progress |= run_loop_pass(self, passes::optimize_if_nz);
	}

	fn pass_info(&self, pass: &str) {
		let (op_count, dloop_count, if_count) = self.stats();
		debug!(
			"running pass {pass} with {op_count} instructions ({dloop_count}) loops and {if_count} ifs"
		);
	}

	fn stats(&self) -> (usize, usize, usize) {
		Self::stats_of(self)
	}

	fn stats_of(ops: &[BrainIr]) -> (usize, usize, usize) {
		let mut op_count = 0;
		let mut dloop_count = 0;
		let mut if_count = 0;

		for op in ops {
			op_count += 1;
			match op {
				BrainIr::DynamicLoop(l) => {
					let (ops, dloops, ifs) = Self::stats_of(l);

					op_count += ops;
					dloop_count += dloops + 1;
					if_count += ifs;
				}
				BrainIr::IfNotZero(l) => {
					let (ops, dloops, ifs) = Self::stats_of(l);

					op_count += ops;
					dloop_count += dloops;
					if_count += ifs + 1;
				}
				_ => {}
			}
		}

		(op_count, dloop_count, if_count)
	}
}

impl Default for Compiler {
	fn default() -> Self {
		Self::new()
	}
}

impl Deref for Compiler {
	type Target = Vec<BrainIr>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl DerefMut for Compiler {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}

impl Extend<BrainIr> for Compiler {
	fn extend<T>(&mut self, iter: T)
	where
		T: IntoIterator<Item = BrainIr>,
	{
		self.inner.extend(iter);
	}
}

impl FromIterator<BrainIr> for Compiler {
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = BrainIr>,
	{
		Self {
			inner: Vec::from_iter(iter),
		}
	}
}

impl<'a> IntoIterator for &'a Compiler {
	type IntoIter = slice::Iter<'a, BrainIr>;
	type Item = &'a BrainIr;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.iter()
	}
}

impl<'a> IntoIterator for &'a mut Compiler {
	type IntoIter = slice::IterMut<'a, BrainIr>;
	type Item = &'a mut BrainIr;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.iter_mut()
	}
}

impl IntoIterator for Compiler {
	type IntoIter = std::vec::IntoIter<BrainIr>;
	type Item = BrainIr;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.into_iter()
	}
}
