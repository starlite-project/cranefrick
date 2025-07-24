#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod cfg;
mod domtree;
mod ion;
#[macro_use]
mod index;
mod postorder;
mod registers;
mod ssa;

use core::hash::BuildHasherDefault;

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

type FxHashMap<K, V> = hashbrown::HashMap<K, V, BuildHasherDefault<FxHasher>>;
type FxHashSet<V> = hashbrown::HashSet<V, BuildHasherDefault<FxHasher>>;
