mod opt;

use alloc::vec::Vec;
use core::{
	ops::{Deref, DerefMut},
	slice,
};

use cranefrick_hlir::BrainHlir;
use serde::{Deserialize, Serialize};

use self::opt::run_peephole_pass;
use super::BrainMlir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Compiler {
	inner: Vec<BrainMlir>,
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

	pub fn push(&mut self, i: BrainMlir) {
		self.inner.push(i);
	}

	pub fn optimize(&mut self) {
		let mut progress = self.optimization_pass();

		while progress {
			progress = self.optimization_pass();
		}
	}

	fn optimization_pass(&mut self) -> bool {
		let mut progress = false;

		self.run_all_passes(&mut progress);

		progress
	}

	fn run_all_passes(&mut self, progress: &mut bool) {
		*progress |= run_peephole_pass(&mut *self, self::opt::passes::combine_instructions);
	}
}

impl Default for Compiler {
	fn default() -> Self {
		Self::new()
	}
}

impl Deref for Compiler {
	type Target = Vec<BrainMlir>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl DerefMut for Compiler {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}

impl Extend<BrainMlir> for Compiler {
	fn extend<T>(&mut self, iter: T)
	where
		T: IntoIterator<Item = BrainMlir>,
	{
		self.inner.extend(iter);
	}
}

impl Extend<BrainHlir> for Compiler {
	fn extend<T>(&mut self, iter: T)
	where
		T: IntoIterator<Item = BrainHlir>,
	{
		self.extend(iter.into_iter().map(BrainMlir::from));
	}
}

impl FromIterator<BrainMlir> for Compiler {
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = BrainMlir>,
	{
		Self {
			inner: Vec::from_iter(iter),
		}
	}
}

impl FromIterator<BrainHlir> for Compiler {
	fn from_iter<T: IntoIterator<Item = BrainHlir>>(iter: T) -> Self {
		iter.into_iter().map(BrainMlir::from).collect::<Self>()
	}
}

impl<'a> IntoIterator for &'a Compiler {
	type IntoIter = slice::Iter<'a, BrainMlir>;
	type Item = &'a BrainMlir;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.iter()
	}
}

impl<'a> IntoIterator for &'a mut Compiler {
	type IntoIter = slice::IterMut<'a, BrainMlir>;
	type Item = &'a mut BrainMlir;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.iter_mut()
	}
}

impl IntoIterator for Compiler {
	type IntoIter = alloc::vec::IntoIter<BrainMlir>;
	type Item = BrainMlir;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.into_iter()
	}
}
