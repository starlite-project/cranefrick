use alloc::vec::Vec;

use super::{
	Block, Function, Instruction, ProgramPoint, RegisterAllocError, VecExt as _, domtree, postorder,
};

#[derive(Debug, Default)]
pub struct CfgInfoContext {
	visited: Vec<bool>,
	block_to_rpo: Vec<Option<u32>>,
	backedge: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct CfgInfo {
	pub postorder: Vec<Block>,
	pub domtree: Vec<Block>,
	pub instruction_block: Vec<Block>,
	pub block_entry: Vec<ProgramPoint>,
	pub block_exit: Vec<ProgramPoint>,
	pub approx_loop_depth: Vec<u32>,
}

impl CfgInfo {
	pub fn new<F: Function>(f: &F) -> Result<Self, RegisterAllocError> {
		let mut ctx = CfgInfoContext::default();
		let mut this = Self::default();
		this.init(f, &mut ctx)?;
		Ok(this)
	}

	pub fn init<F: Function>(
		&mut self,
		f: &F,
		ctx: &mut CfgInfoContext,
	) -> Result<(), RegisterAllocError> {
		let nb = f.block_count();

		postorder::calculate(
			nb,
			f.entry_block(),
			&mut ctx.visited,
			&mut self.postorder,
			|block| f.block_successors(block),
		)?;

		domtree::calculate(
			nb,
			|block| f.block_predecessors(block),
			&self.postorder,
			&mut ctx.block_to_rpo,
			&mut self.domtree,
			f.entry_block(),
		);

		let inst_block = self
			.instruction_block
			.repopulated(f.instruction_count(), Block::invalid());
		let block_entry = self
			.block_entry
			.repopulated(nb, ProgramPoint::before(Instruction::invalid()));
		let block_exit = self
			.block_exit
			.repopulated(nb, ProgramPoint::before(Instruction::invalid()));

		let (backedge_in, backedge_out) = ctx.backedge.repopulated(nb * 2, 0).split_at_mut(nb);

		for block in 0..f.block_count() {
			let block = Block::new(block);
			for inst in f.block_instructions(block) {
				inst_block[inst.index()] = block;
			}

			block_entry[block.index()] = ProgramPoint::before(f.block_instructions(block).first());
			block_exit[block.index()] = ProgramPoint::after(f.block_instructions(block).last());

			let predecessors =
				f.block_predecessors(block).len() + usize::from(block == f.entry_block());

			if predecessors > 1 {
				for &pred in f.block_predecessors(block) {
					let succs = f.block_successors(block).len();
					if succs > 1 {
						return Err(RegisterAllocError::CritEdge(pred, block));
					}
				}
			}

			let mut require_no_branch_args = false;
			for &succ in f.block_successors(block) {
				let preds = f.block_predecessors(succ).len() + usize::from(succ == f.entry_block());
				if preds > 1 {
					require_no_branch_args = false;
					break;
				}
			}

			if require_no_branch_args {
				let last = f.block_instructions(block).last();
				if !f.instruction_operands(last).is_empty() {
					return Err(RegisterAllocError::DisallowedBranchArg(last));
				}
			}

			for &succ in f.block_successors(block) {
				if succ.index() <= block.index() {
					backedge_in[succ.index()] += 1;
					backedge_out[block.index()] += 1;
				}
			}
		}

		let approx_loop_depth = self.approx_loop_depth.cleared();
		let mut backedge_stack = Vec::<u32>::new();
		let mut cur_depth = 0;
		for block in 0..nb {
			if backedge_in[block] > 0 {
				cur_depth += 1;
				backedge_stack.push(backedge_in[block]);
			}

			approx_loop_depth.push(cur_depth);

			while !backedge_stack.is_empty() && backedge_out[block] > 0 {
				backedge_out[block] -= 1;
				*backedge_stack.last_mut().unwrap() -= 1;
				if matches!(backedge_stack.last().unwrap(), 0) {
					cur_depth -= 1;
					backedge_stack.pop();
				}
			}
		}

		Ok(())
	}

	pub fn dominates(&self, a: Block, b: Block) -> bool {
		domtree::dominates(&self.domtree[..], a, b)
	}
}
