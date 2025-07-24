use alloc::vec::Vec;

use super::{Block, VecExt as _};

fn merge_sets(
	idom: &[Block],
	block_to_rpo: &[Option<u32>],
	mut node1: Block,
	mut node2: Block,
) -> Block {
	while node1 != node2 {
		if node1.is_invalid() || node2.is_invalid() {
			return Block::invalid();
		}

		let rpo1 = block_to_rpo[node1.index()].unwrap();
		let rpo2 = block_to_rpo[node2.index()].unwrap();
		if rpo1 > rpo2 {
			node1 = idom[node1.index()];
		} else if rpo2 > rpo1 {
			node2 = idom[node2.index()];
		}
	}

	debug_assert_eq!(node1, node2);
	node1
}

pub fn dominates(idom: &[Block], a: Block, mut b: Block) -> bool {
	loop {
		if a == b {
			return true;
		}

		if b.is_invalid() {
			return false;
		}

		b = idom[b.index()];
	}
}

pub fn calculate<'a>(
	num_blocks: usize,
	preds: impl Fn(Block) -> &'a [Block],
	post_ord: &[Block],
	block_to_rpo_scratch: &mut Vec<Option<u32>>,
	out: &mut Vec<Block>,
	start: Block,
) {
	let block_to_rpo = block_to_rpo_scratch.repopulated(num_blocks, None);
	for (i, rpo_block) in post_ord.iter().rev().enumerate() {
		block_to_rpo[rpo_block.index()] = Some(i as u32);
	}

	let idom = out.repopulated(num_blocks, Block::invalid());
	idom[start.index()] = start;

	let mut changed = true;
	while changed {
		changed = false;

		for &node in post_ord.iter().rev() {
			let rponum = block_to_rpo[node.index()].unwrap();

			let mut parent = Block::invalid();
			for &pred in preds(node) {
				let Some(pred_rpo) = block_to_rpo[pred.index()] else {
					continue;
				};

				if pred_rpo < rponum {
					parent = pred;
					break;
				}
			}

			if parent.is_valid() {
				for &pred in preds(node) {
					if pred == parent {
						continue;
					}

					if idom[pred.index()].is_invalid() {
						continue;
					}

					parent = merge_sets(idom, &block_to_rpo[..], parent, pred);
				}
			}

			if parent.is_valid() && parent != idom[node.index()] {
				idom[node.index()] = parent;
				changed = true;
			}
		}
	}

	idom[start.index()] = Block::invalid();
}
