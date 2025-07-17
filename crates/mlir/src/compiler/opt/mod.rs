mod change;
pub mod passes;

use alloc::vec::Vec;

pub use self::change::*;
use crate::BrainMlir;

pub fn run_peephole_pass<F, const N: usize>(v: &mut Vec<BrainMlir>, pass: F) -> bool
where
	F: Fn(&[BrainMlir; N]) -> Option<Change> + Copy,
{
	let mut i = 0;

	let mut progress = false;

	while v.len() >= N && i < v.len() - (N - 1) {
		let window = core::array::from_fn(|index| v[i + index]);

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

	progress
}
