mod change;
pub mod passes;

use alloc::vec::Vec;

pub use self::change::*;
use crate::BrainMlir;

pub fn run_loop_pass<F>(v: &mut Vec<BrainMlir>, pass: F) -> bool
where
	F: Fn(&[BrainMlir]) -> Option<Change> + Copy,
{
	run_peephole_pass(v, |ops: &[BrainMlir; 1]| match &ops[0] {
		BrainMlir::DynamicLoop(i) => pass(i),
		_ => None,
	})
}

pub fn run_peephole_pass<F, const N: usize>(v: &mut Vec<BrainMlir>, pass: F) -> bool
where
	F: Fn(&[BrainMlir; N]) -> Option<Change> + Copy,
{
	let mut i = 0;

	let mut progress = false;

	while v.len() >= N && i < v.len() - (N - 1) {
		let window = core::array::from_fn(|index| v[i + index].clone());

		let change = pass(&window);

		let changed = if let Some(change) = change {
			change.apply(v, i, N);
			true
		} else {
			false
		};

		if changed {
			progress = true;
		} else {
			i += 1;
		}
	}

	for i in v {
		if let BrainMlir::DynamicLoop(instrs) = i {
			progress |= run_peephole_pass::<_, N>(instrs, pass);
		}
	}

	progress
}
