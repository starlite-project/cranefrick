use alloc::vec;

use tracing::trace;

use super::{
	Block, Function, FunctionExt as _, FxHashSet, Instruction, OperandType, RegisterAllocError,
	VirtualRegister, cfg::CfgInfo,
};

pub fn validate_ssa<F: Function>(f: &F, cfg_info: &CfgInfo) -> Result<(), RegisterAllocError> {
	let mut defined_in = vec![Block::invalid(); f.virtual_register_count()];
	for block in f.blocks() {
		let mut def = |vreg: VirtualRegister, inst| {
			if vreg.virtual_register() >= defined_in.len() {
				trace!("virtual registers not numbered consecutively {vreg:?}");
				return Err(RegisterAllocError::SSA(vreg, inst));
			}

			if defined_in[vreg.virtual_register()].is_valid() {
				trace!("multiple def constraints for {vreg:?}");
				Err(RegisterAllocError::SSA(vreg, inst))
			} else {
				defined_in[vreg.virtual_register()] = block;
				Ok(())
			}
		};

		for param in f.block_parameters(block).iter().copied() {
			def(param, Instruction::invalid())?;
		}

		for inst in f.block_instructions(block) {
			for operand in f.instruction_operands(inst) {
				if matches!(operand.ty(), OperandType::Def) {
					def(operand.virtual_register(), inst)?;
				}
			}
		}
	}

	let mut local = FxHashSet::default();
	for block in f.blocks() {
		local.clear();
		local.extend(f.block_parameters(block));

		for iix in f.block_instructions(block) {
			let operands = f.instruction_operands(iix);
			for operand in operands {
				if operand.as_fixed_nonallocatable().is_some() {
					continue;
				}

				match operand.ty() {
					OperandType::Use => {
						let def_block = defined_in[operand.virtual_register().virtual_register()];
						let okay = def_block.is_valid()
							&& if def_block == block {
								local.contains(&operand.virtual_register())
							} else {
								cfg_info.dominates(def_block, block)
							};
						if !okay {
							trace!("invalid use {:?}", operand.virtual_register());
							return Err(RegisterAllocError::SSA(operand.virtual_register(), iix));
						}
					}
					OperandType::Def => {}
				}
			}

			for operand in operands {
				if matches!(operand.ty(), OperandType::Def) {
					local.insert(operand.virtual_register());
				}
			}
		}
	}

	for block in f.blocks() {
		let insts = f.block_instructions(block);
		for inst in insts {
			if inst == insts.last() {
				if !(f.is_branch(inst) || f.is_return(inst)) {
					trace!("block {block:?} is not terminated by a branch or return");
					return Err(RegisterAllocError::BB(block));
				}

				if f.is_branch(inst) {
					for (i, &succ) in f.block_successors(block).iter().enumerate() {
						let blockparams_in = f.block_parameters(succ);
						let blockparams_out = f.branch_block_parameters(block, inst, i);
						if blockparams_in.len() != blockparams_out.len() {
							trace!(
								"mismatch on block params, found {} expected {}",
								blockparams_out.len(),
								blockparams_in.len()
							);
							return Err(RegisterAllocError::Branch(inst));
						}
					}
				}
			} else if f.is_branch(inst) || f.is_return(inst) {
				trace!("block terminator found in the middle of a block");
				return Err(RegisterAllocError::BB(block));
			}
		}
	}

	if !f.block_parameters(f.entry_block()).is_empty() {
		trace!("entry block contains block args");
		return Err(RegisterAllocError::BB(f.entry_block()));
	}

	Ok(())
}
