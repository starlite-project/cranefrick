#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod cfg;
mod domtree;
pub mod fastalloc;
pub mod ion;
#[macro_use]
mod index;
mod postorder;
mod registers;
mod ssa;

use core::{
	error::Error as CoreError,
	fmt::{Display, Formatter, Result as FmtResult},
	hash::BuildHasherDefault,
};

use rustc_hash::FxHasher;

pub use self::{
	index::{Block, Instruction, InstructionRange, InstructionRangeIter},
	registers::*,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct RegisterAllocOptions {
	pub validate_ssa: bool,
	pub algorith: AllocAlgorithm,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum AllocAlgorithm {
	#[default]
	Ion,
	Fastalloc,
}

#[derive(Debug, Clone)]
#[expect(clippy::upper_case_acronyms)]
pub enum RegisterAllocError {
	CritEdge(Block, Block),
	SSA(VirtualRegister, Instruction),
	BB(Block),
	Branch(Instruction),
	EntryLivein,
	DisallowedBranchArg(Instruction),
	TooManyLiveRegisters,
	TooManyOperands,
}

impl Display for RegisterAllocError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str(match self {
			Self::CritEdge(..) => "critical edge is not split between given blocks",
			Self::SSA(..) => "invalid SSA for given virtual register at given instruction",
			Self::BB(..) => "invalid basic block",
			Self::Branch(..) => "invalid branch",
			Self::EntryLivein => "a vreg is live-in on entry",
			Self::DisallowedBranchArg(..) => "a branch has non-blockparam arg(s) and at least one of the successor blocks has more than one predecessor",
			Self::TooManyLiveRegisters => "too many pinned vregs",
			Self::TooManyOperands => "too many operands on a single instruction"
		})
	}
}

impl CoreError for RegisterAllocError {}

pub(crate) trait VecExt<T> {
	fn repopulated(&mut self, len: usize, value: T) -> &mut [T]
	where
		T: Clone;

	fn cleared(&mut self) -> &mut Self;

	fn preallocated(&mut self, cap: usize) -> &mut Self;
}

impl<T> VecExt<T> for alloc::vec::Vec<T> {
	fn repopulated(&mut self, len: usize, value: T) -> &mut [T]
	where
		T: Clone,
	{
		self.clear();
		self.resize(len, value);
		self
	}

	fn cleared(&mut self) -> &mut Self {
		self.clear();
		self
	}

	fn preallocated(&mut self, cap: usize) -> &mut Self {
		self.clear();
		self.reserve(cap);
		self
	}
}

pub trait Function {
	fn instruction_count(&self) -> usize;

	fn block_count(&self) -> usize;

	fn entry_block(&self) -> Block;

	fn block_instructions(&self, block: Block) -> InstructionRange;

	fn block_successors(&self, block: Block) -> &[Block];

	fn block_predecessors(&self, block: Block) -> &[Block];

	fn block_parameters(&self, block: Block) -> &[VirtualRegister];

	fn is_return(&self, inst: Instruction) -> bool;

	fn is_branch(&self, inst: Instruction) -> bool;

	fn branch_block_parameters(
		&self,
		block: Block,
		inst: Instruction,
		succ_idx: usize,
	) -> &[VirtualRegister];

	fn instruction_operands(&self, inst: Instruction) -> &[Operand];

	fn instruction_clobbers(&self, inst: Instruction) -> PhysicalRegisterSet;

	fn virtual_register_count(&self) -> usize;

	fn spillslot_size(&self, class: RegisterClass) -> usize;

	fn multi_spillslot_named_by_last_slot(&self) -> bool {
		false
	}

	fn debug_value_labels(&self) -> &[(VirtualRegister, Instruction, Instruction, u32)] {
		&[]
	}

	fn allow_multiple_virtual_register_defs(&self) -> bool {
		false
	}
}

pub(crate) trait FunctionExt: Function {
	fn blocks(&self) -> impl Iterator<Item = Block>;
}

impl<T: Function> FunctionExt for T {
	fn blocks(&self) -> impl Iterator<Item = Block> {
		(0..self.block_count()).map(Block::new)
	}
}

type FxHashMap<K, V> = hashbrown::HashMap<K, V, BuildHasherDefault<FxHasher>>;
type FxHashSet<V> = hashbrown::HashSet<V, BuildHasherDefault<FxHasher>>;
