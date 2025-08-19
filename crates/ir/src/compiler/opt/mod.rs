mod change;
pub mod passes;
mod utils;

pub use self::change::*;
use crate::BrainIr;

pub fn run_loop_pass<F>(v: &mut Vec<BrainIr>, pass: F) -> bool
where
	F: Fn(&[BrainIr]) -> Option<Change> + Copy,
{
	run_peephole_pass(v, |ops: &[BrainIr; 1]| match &ops[0] {
		BrainIr::DynamicLoop(i) => pass(i),
		_ => None,
	})
}

pub fn run_peephole_pass<F, const N: usize>(v: &mut Vec<BrainIr>, pass: F) -> bool
where
	F: Fn(&[BrainIr; N]) -> Option<Change> + Copy,
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

	v.iter_mut()
		.filter_map(BrainIr::child_ops_mut)
		.for_each(|ops| {
			progress |= run_peephole_pass::<_, N>(ops, pass);
		});

	progress
}
