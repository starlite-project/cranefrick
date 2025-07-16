#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

extern crate alloc;

mod mir;
mod opt;

use alloc::vec::Vec;

use cranefrick_hir::BrainHir;
use serde::{Deserialize, Serialize};

pub use self::mir::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compiler {
	inner: Vec<BrainMir>,
}

impl Compiler {
	#[must_use]
	pub const fn new() -> Self {
		Self { inner: Vec::new() }
	}

	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			inner: Vec::with_capacity(capacity),
		}
	}

	pub fn push(&mut self, i: BrainMir) {
		self.inner.push(i);
	}

	pub fn optimize(&mut self) {}
}

impl Default for Compiler {
	fn default() -> Self {
		Self::new()
	}
}

impl Extend<BrainMir> for Compiler {
	fn extend<T>(&mut self, iter: T)
	where
		T: IntoIterator<Item = BrainMir>,
	{
		self.inner.extend(iter);
	}
}

impl Extend<BrainHir> for Compiler {
	fn extend<T>(&mut self, iter: T)
	where
		T: IntoIterator<Item = BrainHir>,
	{
		self.extend(iter.into_iter().map(BrainMir::from));
	}
}

impl FromIterator<BrainMir> for Compiler {
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = BrainMir>,
	{
		Self {
			inner: Vec::from_iter(iter),
		}
	}
}

impl FromIterator<BrainHir> for Compiler {
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = BrainHir>,
	{
		iter.into_iter().map(BrainMir::from).collect::<Self>()
	}
}

impl IntoIterator for Compiler {
	type IntoIter = alloc::vec::IntoIter<BrainMir>;
	type Item = BrainMir;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.into_iter()
	}
}
