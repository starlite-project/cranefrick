mod change;
pub mod passes;
mod utils;

use alloc::vec::Vec;
use core::array;

use frick_ir::BrainIr;

pub use self::change::*;

#[tracing::instrument(skip_all)]
pub fn run_loop_pass(v: &mut Vec<BrainIr>, pass: impl LoopPass) -> bool {
	run_peephole_pass_inner(v, |ops| match ops {
		[BrainIr::DynamicLoop(i)] => pass(i),
		_ => None,
	})
}

#[tracing::instrument(skip_all)]
pub fn run_peephole_pass<const N: usize>(v: &mut Vec<BrainIr>, pass: impl PeepholePass<N>) -> bool {
	run_peephole_pass_inner(v, pass)
}

fn run_peephole_pass_inner<const N: usize>(
	v: &mut Vec<BrainIr>,
	pass: impl PeepholePass<N>,
) -> bool {
	let mut i = 0;
	let mut progress = false;

	while v.len() >= N && i < v.len() - (N - 1) {
		let change = {
			let window = array::from_fn(|index| &v[i + index]);

			pass(window)
		};

		let Some(change) = change else {
			i += 1;
			continue;
		};

		change.apply::<N>(v, i);
		progress = true;
	}

	v.iter_mut()
		.filter_map(BrainIr::child_ops_mut)
		.for_each(|child| {
			progress |= run_peephole_pass_inner::<N>(child, pass);
		});

	progress
}

pub(crate) trait PeepholePass<const N: usize>:
	Copy + Fn([&BrainIr; N]) -> Option<Change>
{
}

impl<T, const N: usize> PeepholePass<N> for T where T: Copy + Fn([&BrainIr; N]) -> Option<Change> {}

pub(crate) trait LoopPass: Copy + Fn(&[BrainIr]) -> Option<Change> {}

impl<T> LoopPass for T where T: Copy + Fn(&[BrainIr]) -> Option<Change> {}
