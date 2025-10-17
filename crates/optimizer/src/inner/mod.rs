mod change;
pub mod passes;
mod utils;

use frick_ir::BrainIr;

pub use self::change::*;

pub fn run_loop_pass(
	v: &mut Vec<BrainIr>,
	pass: impl Fn(&[BrainIr]) -> Option<Change> + Copy,
) -> bool {
	run_peephole_pass(v, |ops: [&BrainIr; 1]| match &ops[0] {
		BrainIr::DynamicLoop(i) => pass(i),
		_ => None,
	})
}

pub fn run_peephole_pass<const N: usize>(
	v: &mut Vec<BrainIr>,
	pass: impl Fn([&BrainIr; N]) -> Option<Change> + Copy,
) -> bool {
	let mut i = 0;
	let mut progress = false;

	while v.len() >= N && i < v.len() - (N - 1) {
		let change = {
			let window = std::array::from_fn(|index| &v[i + index]);

			pass(window)
		};

		if let Some(change) = change {
			change.apply::<N>(v, i);
			progress = true;
		} else {
			i += 1;
		}
	}

	v.iter_mut()
		.filter_map(BrainIr::child_ops_mut)
		.for_each(|child| {
			progress |= run_peephole_pass::<N>(child, pass);
		});

	progress
}
