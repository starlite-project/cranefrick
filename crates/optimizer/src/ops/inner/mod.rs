mod change;
pub mod passes;
mod utils;

use std::array;

use frick_operations::BrainOperation;

pub use self::change::*;

#[tracing::instrument(skip_all)]
pub fn run_loop_pass(v: &mut Vec<BrainOperation>, pass: impl LoopPass) -> bool {
	run_peephole_pass_inner(v, |ops| {
		let [op] = ops;

		let child_ops = op.child_ops()?;

		pass(child_ops)
	})
}

#[tracing::instrument(skip_all)]
pub fn run_peephole_pass<const N: usize>(
	v: &mut Vec<BrainOperation>,
	pass: impl PeepholePass<N>,
) -> bool {
	run_peephole_pass_inner(v, pass)
}

fn run_peephole_pass_inner<const N: usize>(
	v: &mut Vec<BrainOperation>,
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
		.filter_map(|op| op.child_ops_mut())
		.for_each(|child| {
			progress |= run_peephole_pass_inner(child, pass);
		});

	progress
}

pub(crate) trait PeepholePass<const N: usize>:
	Copy + Fn([&BrainOperation; N]) -> Option<Change>
{
}

impl<T, const N: usize> PeepholePass<N> for T where
	T: Copy + Fn([&BrainOperation; N]) -> Option<Change>
{
}

pub(crate) trait LoopPass: Copy + Fn(&[BrainOperation]) -> Option<Change> {}

impl<T> LoopPass for T where T: Copy + Fn(&[BrainOperation]) -> Option<Change> {}
