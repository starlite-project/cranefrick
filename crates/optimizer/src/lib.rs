#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod inner;

use std::{
	iter,
	ops::{Deref, DerefMut},
};

use frick_ir::BrainIr;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use tracing_indicatif::{span_ext::IndicatifSpanExt as _, style::ProgressStyle};

use self::inner::{passes, run_loop_pass, run_peephole_pass};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Optimizer {
	inner: Vec<BrainIr>,
}

impl Optimizer {
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

	#[tracing::instrument("optimize mlir", skip(self), fields(indicatif.pb_show = tracing::field::Empty))]
	pub fn run(&mut self) {
		let mut iteration = 0;

		let mut progress = self.optimization_pass(iteration);

		while progress {
			iteration += 1;
			progress = self.optimization_pass(iteration);
		}

		info!(iterations = iteration, "finished optimizing mlir");
	}

	#[tracing::instrument("run passes", skip(self), fields(indicatif.pb_show = tracing::field::Empty))]
	fn optimization_pass(&mut self, iteration: usize) -> bool {
		let span = tracing::Span::current();
		let mut progress = false;

		span.pb_set_style(
			&ProgressStyle::with_template(
				"{span_child_prefix}{spinner} {span_name}({span_fields}) [{bar}] ({pos}/{len}) [{elapsed_precise}]",
			)
			.unwrap()
			.progress_chars("#>-"),
		);
		span.pb_set_length(45);

		self.run_all_passes(&mut progress);

		progress
	}

	fn run_all_passes(&mut self, progress: &mut bool) {
		let span = tracing::Span::current();

		self.pass_info("combine relavent instructions");
		*progress |= run_peephole_pass(self, passes::optimize_consecutive_instructions);
		span.pb_inc(1);

		self.pass_info("add relavent offsets");
		*progress |= run_peephole_pass(self, passes::add_offsets);
		span.pb_inc(1);

		self.pass_info("fix boundary instructions");
		*progress |= run_peephole_pass(self, passes::optimize_initial_sets);
		*progress |= run_peephole_pass(self, passes::fix_boundary_instructions);
		span.pb_inc(2);

		self.pass_info("optimize clear cell instructions");
		*progress |= run_loop_pass(self, passes::clear_cell);
		span.pb_inc(1);

		self.pass_info("optimize set-based instructions");
		*progress |= run_peephole_pass(self, passes::optimize_sets);
		span.pb_inc(1);

		self.pass_info("optimize find zero instructions");
		*progress |= run_loop_pass(self, passes::optimize_find_zero);
		span.pb_inc(1);

		self.pass_info("remove no-op instructions");
		*progress |= run_peephole_pass(self, passes::remove_noop_instructions);
		*progress |= run_loop_pass(self, passes::unroll_noop_loop);
		span.pb_inc(2);

		self.pass_info("remove unreachable loops");
		*progress |= run_peephole_pass(self, passes::remove_unreachable_loops);
		span.pb_inc(1);

		self.pass_info("remove infinite loops");
		*progress |= run_loop_pass(self, passes::remove_infinite_loops);
		span.pb_inc(1);

		self.pass_info("remove empty loops");
		*progress |= run_loop_pass(self, passes::remove_empty_loops);
		span.pb_inc(1);

		self.pass_info("unroll no-move dynamic loops");
		*progress |= run_peephole_pass(self, passes::unroll_basic_dynamic_loop);
		span.pb_inc(1);

		self.pass_info("sort cell changes");
		*progress |= run_peephole_pass(self, passes::sort_changes::<8>);
		*progress |= run_peephole_pass(self, passes::sort_changes::<7>);
		*progress |= run_peephole_pass(self, passes::sort_changes::<6>);
		*progress |= run_peephole_pass(self, passes::sort_changes::<5>);
		*progress |= run_peephole_pass(self, passes::sort_changes::<4>);
		*progress |= run_peephole_pass(self, passes::sort_changes::<3>);
		*progress |= run_peephole_pass(self, passes::sort_changes::<2>);
		span.pb_inc(7);

		self.pass_info("optimize scale and shift value instructions");
		*progress |= run_loop_pass(self, passes::optimize_move_value_from_loop);
		*progress |= run_peephole_pass(self, passes::optimize_move_value);
		*progress |= run_peephole_pass(self, passes::optimize_move_value_from_duplicate_cells);
		*progress |= run_peephole_pass(self, passes::optimize_take_value);
		*progress |= run_peephole_pass(self, passes::optimize_fetch_value);
		*progress |= run_peephole_pass(self, passes::optimize_replace_value);
		*progress |= run_peephole_pass(self, passes::optimize_copy_value);
		span.pb_inc(7);

		self.pass_info("optimize write calls");
		*progress |= run_peephole_pass(self, passes::optimize_writes);
		*progress |= run_peephole_pass(self, passes::optimize_changes_and_writes);
		*progress |= run_peephole_pass(self, passes::optimize_offset_writes);
		span.pb_inc(3);

		self.pass_info("remove redundant take instructions");
		*progress |= run_peephole_pass(self, passes::remove_redundant_shifts);
		span.pb_inc(1);

		self.pass_info("optimize constant shifts");
		*progress |= run_peephole_pass(self, passes::optimize_constant_shifts);
		span.pb_inc(1);

		self.pass_info("remove unnecessary offsets");
		*progress |= run_peephole_pass(self, passes::remove_offsets);
		span.pb_inc(1);

		self.pass_info("optimize sub cell");
		*progress |= run_loop_pass(self, passes::optimize_sub_cell_at);
		*progress |= run_peephole_pass(self, passes::optimize_sub_cell_from);
		*progress |= run_peephole_pass(self, passes::optimize_sub_cell_from_with_set);
		*progress |= run_peephole_pass(self, passes::optimize_constant_sub);
		span.pb_inc(4);

		self.pass_info("optimize if not zero");
		*progress |= run_loop_pass(self, passes::optimize_if_nz);
		*progress |= run_peephole_pass(self, passes::optimize_if_nz_when_zeroing);
		span.pb_inc(2);

		self.pass_info("optimize duplicate cell");
		*progress |= run_loop_pass(self, passes::optimize_duplicate_cell);
		*progress |= run_peephole_pass(self, passes::optimize_duplicate_cell_vectorization);
		*progress |= run_peephole_pass(self, passes::unroll_constant_duplicate_cell);
		span.pb_inc(3);

		self.pass_info("optimize memory operations");
		*progress |= run_peephole_pass(self, passes::optimize_mem_sets);
		*progress |= run_peephole_pass(self, passes::optimize_mem_set_move_change);
		span.pb_inc(2);

		self.pass_info("unroll certain if nz");
		*progress |= run_peephole_pass(self, passes::unroll_constant_if_nz);
		span.pb_inc(1);
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

impl Default for Optimizer {
	fn default() -> Self {
		Self::new()
	}
}

impl Deref for Optimizer {
	type Target = Vec<BrainIr>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl DerefMut for Optimizer {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}

impl FromIterator<BrainIr> for Optimizer {
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = BrainIr>,
	{
		Self {
			inner: iter::once(BrainIr::boundary())
				.chain(iter)
				.chain(iter::once(BrainIr::boundary()))
				.collect(),
		}
	}
}
