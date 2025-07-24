use alloc::vec;

use super::{
	Block, Function, FxHashSet, Instruction, OperandType, RegisterAllocError, VirtualRegister,
	cfg::CfgInfo,
};

pub fn validate_ssa<F: Function>(f: &F, cfg_info: &CfgInfo) -> Result<(), RegisterAllocError> {
	let mut defined_in = vec![Block::invalid(); f.virtual_register_count()];
	for block in 0..f.block_count() {}

	Ok(())
}
