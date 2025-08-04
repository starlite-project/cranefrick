#[cfg(feature = "alloc")]
use alloc::vec;
use core::{
	iter::{Enumerate, FusedIterator},
	marker::PhantomData,
	slice,
};

use super::EntityRef;

#[repr(transparent)]
pub struct Iter<'a, K: EntityRef, V: 'a> {
	inner: Enumerate<slice::Iter<'a, V>>,
	marker: PhantomData<K>,
}

impl<'a, K: EntityRef, V> Iter<'a, K, V> {
	pub fn new(slice: &'a [V]) -> Self {
		Self {
			inner: slice.iter().enumerate(),
			marker: PhantomData,
		}
	}
}

impl<K: EntityRef, V> DoubleEndedIterator for Iter<'_, K, V> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.inner.next_back().map(mapper)
	}
}

impl<K: EntityRef, V> ExactSizeIterator for Iter<'_, K, V> {
	fn len(&self) -> usize {
		self.inner.len()
	}
}

impl<K: EntityRef, V> FusedIterator for Iter<'_, K, V> {}

impl<'a, K: EntityRef, V> Iterator for Iter<'a, K, V> {
	type Item = (K, &'a V);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(mapper)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	fn count(self) -> usize {
		self.len()
	}

	fn last(mut self) -> Option<Self::Item> {
		self.next_back()
	}
}

#[repr(transparent)]
pub struct IterMut<'a, K: EntityRef, V: 'a> {
	inner: Enumerate<slice::IterMut<'a, V>>,
	marker: PhantomData<K>,
}

impl<'a, K: EntityRef, V> IterMut<'a, K, V> {
	pub fn new(slice: &'a mut [V]) -> Self {
		Self {
			inner: slice.iter_mut().enumerate(),
			marker: PhantomData,
		}
	}
}

impl<K: EntityRef, V> DoubleEndedIterator for IterMut<'_, K, V> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.inner.next_back().map(mapper)
	}
}

impl<K: EntityRef, V> ExactSizeIterator for IterMut<'_, K, V> {
	fn len(&self) -> usize {
		self.inner.len()
	}
}

impl<K: EntityRef, V> FusedIterator for IterMut<'_, K, V> {}

impl<'a, K: EntityRef, V> Iterator for IterMut<'a, K, V> {
	type Item = (K, &'a mut V);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(mapper)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	fn count(self) -> usize {
		self.len()
	}

	fn last(mut self) -> Option<Self::Item> {
		self.next_back()
	}
}

#[cfg(feature = "alloc")]
#[repr(transparent)]
pub struct IntoIter<K: EntityRef, V> {
	inner: Enumerate<vec::IntoIter<V>>,
	marker: PhantomData<K>,
}

#[cfg(feature = "alloc")]
impl<K: EntityRef, V> IntoIter<K, V> {
	#[must_use]
	pub fn new(v: vec::Vec<V>) -> Self {
		Self {
			inner: v.into_iter().enumerate(),
			marker: PhantomData,
		}
	}
}

#[cfg(feature = "alloc")]
impl<K: EntityRef, V> DoubleEndedIterator for IntoIter<K, V> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.inner.next_back().map(mapper)
	}
}

#[cfg(feature = "alloc")]
impl<K: EntityRef, V> ExactSizeIterator for IntoIter<K, V> {
	fn len(&self) -> usize {
		self.inner.len()
	}
}

#[cfg(feature = "alloc")]
impl<K: EntityRef, V> FusedIterator for IntoIter<K, V> {}

#[cfg(feature = "alloc")]
impl<K: EntityRef, V> Iterator for IntoIter<K, V> {
	type Item = (K, V);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(mapper)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	fn count(self) -> usize {
		self.len()
	}

	fn last(mut self) -> Option<Self::Item> {
		self.next_back()
	}
}

fn mapper<K: EntityRef, T>((i, v): (usize, T)) -> (K, T) {
	(K::new(i), v)
}
