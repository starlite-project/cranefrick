use alloc::vec::Vec;
use core::slice;

use super::{Block, RegisterAllocError, VecExt as _};

struct State<'a> {
	block: Block,
	successors: slice::Iter<'a, Block>,
}

pub fn calculate<'a>(
	num_blocks: usize,
	entry: Block,
	visited_scratch: &mut Vec<bool>,
	out: &mut Vec<Block>,
	succ_blocks: impl Fn(Block) -> &'a [Block],
) -> Result<(), RegisterAllocError> {
	let visited = visited_scratch.repopulated(num_blocks, false);
	let mut stack = Vec::<State<'_>>::new();
	out.clear();

	let entry_visit = visited
		.get_mut(entry.index())
		.ok_or(RegisterAllocError::BB(entry))?;
	*entry_visit = true;
	stack.push(State {
		block: entry,
		successors: succ_blocks(entry).iter(),
	});

	while let Some(ref mut state) = stack.last_mut() {
		if let Some(&succ) = state.successors.next() {
			let succ_visited = visited
				.get_mut(succ.index())
				.ok_or(RegisterAllocError::BB(succ))?;

			if !*succ_visited {
				*succ_visited = true;
				stack.push(State {
					block: succ,
					successors: succ_blocks(succ).iter(),
				});
			}
		} else {
			out.push(state.block);
			stack.pop();
		}
	}

	Ok(())
}
