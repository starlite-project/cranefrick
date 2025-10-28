#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;

mod inner;

use alloc::vec::Vec;
use core::{
	iter,
	ops::{Deref, DerefMut},
};

use frick_ir::BrainIr;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, trace_span};
use tracing_indicatif::{span_ext::IndicatifSpanExt as _, style::ProgressStyle};

use self::inner::{LoopPass, PeepholePass, passes, run_loop_pass, run_peephole_pass};

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
		span.pb_set_length(49);

		self.run_all_passes(&mut progress);

		progress
	}

	fn run_all_passes(&mut self, progress: &mut bool) {
		{
			let _guard = self.pass_info("combine relavent instructions", 1);
			run_peephole_pass_with_span(
				"optimize_consecutive_instructions",
				progress,
				self,
				passes::optimize_consecutive_instructions,
			);
		}

		{
			let _guard = self.pass_info("add relative offsets", 1);
			run_peephole_pass_with_span("add_offsets", progress, self, passes::add_offsets);
		}

		{
			let _guard = self.pass_info("fix boundary instructions", 2);
			run_peephole_pass_with_span(
				"optimize_initial_sets",
				progress,
				self,
				passes::optimize_initial_sets,
			);
			run_peephole_pass_with_span(
				"fix_boundary_instructions",
				progress,
				self,
				passes::fix_boundary_instructions,
			);
		}

		{
			let _guard = self.pass_info("optimize clear cell instructions", 1);
			run_loop_pass_with_span("clear_cell", progress, self, passes::clear_cell);
		}

		{
			let _guard = self.pass_info("optimize set-based instructions", 2);
			run_peephole_pass_with_span("optimize_sets", progress, self, passes::optimize_sets);
			run_peephole_pass_with_span(
				"optimize_set_move_op",
				progress,
				self,
				passes::optimize_set_move_op,
			);
		}

		{
			let _guard = self.pass_info("optimize find zero instructions", 1);
			run_loop_pass_with_span(
				"optimize_find_zero",
				progress,
				self,
				passes::optimize_find_zero,
			);
		}

		{
			let _guard = self.pass_info("remove no-op instructions", 1);
			run_peephole_pass_with_span(
				"remove_noop_instructions",
				progress,
				self,
				passes::remove_noop_instructions,
			);
		}

		{
			let _guard = self.pass_info("remove unreachable loops", 1);
			run_peephole_pass_with_span(
				"remove_unreachable_loops",
				progress,
				self,
				passes::remove_unreachable_loops,
			);
		}

		{
			let _guard = self.pass_info("remove infinite loops", 1);
			run_loop_pass_with_span(
				"remove_infinite_loops",
				progress,
				self,
				passes::remove_infinite_loops,
			);
		}

		{
			let _guard = self.pass_info("remove empty loops", 1);
			run_loop_pass_with_span(
				"remove_empty_loops",
				progress,
				self,
				passes::remove_empty_loops,
			);
		}

		{
			let _guard = self.pass_info("unroll no-move dynamic loops", 1);
			run_peephole_pass_with_span(
				"unroll_basic_dynamic_loop",
				progress,
				self,
				passes::unroll_basic_dynamic_loop,
			);
		}

		{
			let _guard = self.pass_info("unroll nested one-op loops", 1);
			run_loop_pass_with_span(
				"unroll_nested_loops",
				progress,
				self,
				passes::unroll_nested_loops,
			);
		}

		{
			let _guard = self.pass_info("sort cell changes", 7);
			run_with_span("sort_changes<8>", || {
				*progress |= run_peephole_pass(self, passes::sort_changes::<8>);
			});
			run_with_span("sort_changes<7>", || {
				*progress |= run_peephole_pass(self, passes::sort_changes::<7>);
			});
			run_with_span("sort_changes<6>", || {
				*progress |= run_peephole_pass(self, passes::sort_changes::<6>);
			});
			run_with_span("sort_changes<5>", || {
				*progress |= run_peephole_pass(self, passes::sort_changes::<5>);
			});
			run_with_span("sort_changes<4>", || {
				*progress |= run_peephole_pass(self, passes::sort_changes::<4>);
			});
			run_with_span("sort_changes<3>", || {
				*progress |= run_peephole_pass(self, passes::sort_changes::<3>);
			});
			run_with_span("sort_changes<2>", || {
				*progress |= run_peephole_pass(self, passes::sort_changes::<2>);
			});
		}

		{
			let _guard = self.pass_info("optimize scale and shift value instructions", 9);
			run_with_span("optimize_move_value_from_loop", || {
				*progress |= run_loop_pass(self, passes::optimize_move_value_from_loop);
			});
			run_with_span("optimize_move_value", || {
				*progress |= run_peephole_pass(self, passes::optimize_move_value);
			});
			run_with_span("optimize_move_value_from_duplicate_cells", || {
				*progress |=
					run_peephole_pass(self, passes::optimize_move_value_from_duplicate_cells);
			});
			run_with_span("optimize_duplicate_cell_replace_from", || {
				*progress |= run_peephole_pass(self, passes::optimize_duplicate_cell_replace_from);
			});
			run_with_span("optimize_take_value", || {
				*progress |= run_peephole_pass(self, passes::optimize_take_value);
			});
			run_with_span("optimize_fetch_value", || {
				*progress |= run_peephole_pass(self, passes::optimize_fetch_value);
			});
			run_with_span("optimize_replace_value", || {
				*progress |= run_peephole_pass(self, passes::optimize_replace_value);
			});
			run_with_span("optimize_copy_value", || {
				*progress |= run_peephole_pass(self, passes::optimize_copy_value);
			});
			run_with_span("optimize_scale_value", || {
				*progress |= run_peephole_pass(self, passes::optimize_scale_value);
			});
		}

		{
			let _guard = self.pass_info("optimize write calls", 5);
			run_with_span("optimize_writes", || {
				*progress |= run_peephole_pass(self, passes::optimize_writes);
			});
			run_with_span("optimize_changes_and_writes", || {
				*progress |= run_peephole_pass(self, passes::optimize_changes_and_writes);
			});
			run_with_span("optimize_offset_writes", || {
				*progress |= run_peephole_pass(self, passes::optimize_offset_writes);
			});
			run_with_span("optimize_change_write_sets", || {
				*progress |= run_peephole_pass(self, passes::optimize_change_write_sets);
			});
			run_peephole_pass_with_span(
				"optimize_boundary_writes",
				progress,
				self,
				passes::optimize_boundary_writes,
			);
		}

		{
			let _guard = self.pass_info("remove redundant take instructions", 1);
			run_with_span("remove_redundant_shifts", || {
				*progress |= run_peephole_pass(self, passes::remove_redundant_shifts);
			});
		}

		{
			let _guard = self.pass_info("optimize constant shifts", 1);
			run_with_span("optimize_constant_shifts", || {
				*progress |= run_peephole_pass(self, passes::optimize_constant_shifts);
			});
		}

		{
			let _guard = self.pass_info("remove unnecessary offsets", 1);
			run_with_span("remove_offsets", || {
				*progress |= run_peephole_pass(self, passes::remove_offsets);
			});
		}

		{
			let _guard = self.pass_info("optimize sub cell", 3);
			run_with_span("optimize_sub_cell_at", || {
				*progress |= run_loop_pass(self, passes::optimize_sub_cell_at);
			});
			run_with_span("optimize_sub_cell_from", || {
				*progress |= run_peephole_pass(self, passes::optimize_sub_cell_from);
			});
			run_with_span("optimize_sub_cell_from_with_set", || {
				*progress |= run_peephole_pass(self, passes::optimize_sub_cell_from_with_set);
			});
		}

		{
			let _guard = self.pass_info("optimize if not zero", 3);
			run_with_span("optimize_if_nz", || {
				*progress |= run_loop_pass(self, passes::optimize_if_nz);
			});
			run_with_span("optimize_if_nz_when_zeroing", || {
				*progress |= run_peephole_pass(self, passes::optimize_if_nz_when_zeroing);
			});
			run_with_span("unroll_if_nz", || {
				*progress |= run_peephole_pass(self, passes::unroll_if_nz);
			});
		}

		{
			let _guard = self.pass_info("optimize duplicate cell", 2);
			run_with_span("optimize_duplicate_cell", || {
				*progress |= run_loop_pass(self, passes::optimize_duplicate_cell);
			});
			run_with_span("unroll_constant_duplicate_cell", || {
				*progress |= run_peephole_pass(self, passes::unroll_constant_duplicate_cell);
			});
		}

		{
			let _guard = self.pass_info("optimize memory operations", 2);
			run_with_span("optimize_mem_sets", || {
				*progress |= run_peephole_pass(self, passes::optimize_mem_sets);
			});
			run_with_span("optimize_mem_set_move_change", || {
				*progress |= run_peephole_pass(self, passes::optimize_mem_set_move_change);
			});
		}

		{
			let _guard = self.pass_info("unroll certain if nz", 1);
			run_with_span("unroll_constant_if_nz", || {
				*progress |= run_peephole_pass(self, passes::unroll_constant_if_nz);
			});
		}
	}

	fn pass_info(&self, pass: &str, count: u64) -> impl Drop + use<> {
		struct DropGuard {
			count: u64,
		}

		impl Drop for DropGuard {
			fn drop(&mut self) {
				tracing::Span::current().pb_inc(self.count);
			}
		}

		let (op_count, dloop_count, if_count) = self.stats();
		debug!(
			"running pass{plural} to {pass} with {op_count} instructions, {dloop_count} loops and {if_count} ifs",
			plural = if matches!(count, 1) { "" } else { "es" }
		);

		DropGuard { count }
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

fn run_with_span(pass_name: &'static str, mut f: impl FnMut()) {
	let span = trace_span!("run_pass", pass = pass_name).entered();
	f();
	drop(span);
}

fn run_peephole_pass_with_span<const N: usize>(
	pass_name: &'static str,
	progress: &mut bool,
	optimizer: &mut Vec<BrainIr>,
	pass: impl PeepholePass<N>,
) {
	let span = trace_span!("run_pass", pass = pass_name).entered();
	*progress |= run_peephole_pass(optimizer, pass);
	drop(span);
}

fn run_loop_pass_with_span(
	pass_name: &'static str,
	progress: &mut bool,
	optimizer: &mut Vec<BrainIr>,
	pass: impl LoopPass,
) {
	let span = trace_span!("run_pass", pass = pass_name).entered();
	*progress |= run_loop_pass(optimizer, pass);
	drop(span);
}
