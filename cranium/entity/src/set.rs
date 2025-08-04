use core::{
	fmt::{Debug, Formatter, Result as FmtResult},
	iter::FusedIterator,
	marker::PhantomData,
};

use cranium_bitset::{CompoundBitSet, CompoundIter};
use serde::{Deserialize, Serialize};

use super::{EntityRef, Keys};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct EntitySet<K: EntityRef> {
	bitset: CompoundBitSet,
	marker: PhantomData<K>,
}

impl<K: EntityRef> EntitySet<K> {
	#[must_use]
	pub fn new() -> Self {
		Self {
			bitset: CompoundBitSet::new(),
			marker: PhantomData,
		}
	}

	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			bitset: CompoundBitSet::with_capacity(capacity),
			marker: PhantomData,
		}
	}

	pub fn ensure_capacity(&mut self, capacity: usize) {
		self.bitset.ensure_capacity(capacity);
	}

	pub fn contains(&self, key: K) -> bool {
		let index = key.index();
		self.bitset.contains(index)
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.bitset.is_empty()
	}

	#[must_use]
	pub fn len(&self) -> usize {
		self.bitset.len()
	}

	pub fn clear(&mut self) {
		self.bitset.clear();
	}

	#[must_use]
	pub fn keys(&self) -> Keys<K> {
		Keys::new(self.bitset.max().map_or(0, |x| x + 1))
	}

	pub fn insert(&mut self, key: K) -> bool {
		let index = key.index();
		self.bitset.insert(index)
	}

	pub fn remove(&mut self, key: K) -> bool {
		let index = key.index();
		self.bitset.remove(index)
	}

	pub fn pop(&mut self) -> Option<K> {
		let index = self.bitset.pop()?;
		Some(K::new(index))
	}

	#[must_use]
	pub const fn iter(&self) -> SetIter<'_, K> {
		SetIter {
			inner: self.bitset.iter(),
			marker: PhantomData,
		}
	}
}

impl<K> Debug for EntitySet<K>
where
	K: Debug + EntityRef,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_set().entries(self.keys()).finish()
	}
}

impl<K: EntityRef> Default for EntitySet<K> {
	fn default() -> Self {
		Self::new()
	}
}

impl<K: EntityRef> Extend<K> for EntitySet<K> {
	fn extend<T>(&mut self, iter: T)
	where
		T: IntoIterator<Item = K>,
	{
		for k in iter {
			self.insert(k);
		}
	}
}

impl<'a, K: EntityRef> IntoIterator for &'a EntitySet<K> {
	type IntoIter = SetIter<'a, K>;
	type Item = K;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

#[repr(transparent)]
pub struct SetIter<'a, K> {
	inner: CompoundIter<'a>,
	marker: PhantomData<K>,
}

impl<K: EntityRef> ExactSizeIterator for SetIter<'_, K> {
	fn len(&self) -> usize {
		self.inner.len()
	}
}

impl<K: EntityRef> FusedIterator for SetIter<'_, K> {}

impl<K: EntityRef> Iterator for SetIter<'_, K> {
	type Item = K;

	fn next(&mut self) -> Option<Self::Item> {
		let index = self.inner.next()?;
		Some(K::new(index))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}
}
