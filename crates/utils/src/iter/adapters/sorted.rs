use alloc::vec::{IntoIter as VecIntoIter, Vec};
use core::{cmp::Ordering, iter::FusedIterator, marker::PhantomData, mem};

use crate::IntoIteratorExt as _;

#[repr(transparent)]
pub struct Sorted<T> {
	pub(crate) iter: VecIntoIter<T>,
}

impl<T: Ord> Sorted<T> {
	pub(crate) fn new(iter: impl IntoIterator<Item = T>) -> Self {
		Self {
			iter: {
				let mut sorter = Vec::from_iter(iter);

				sorter.sort();

				sorter.into_iter()
			},
		}
	}
}

impl<T> DoubleEndedIterator for Sorted<T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter.next_back()
	}
}

impl<T> ExactSizeIterator for Sorted<T> {
	fn len(&self) -> usize {
		self.iter.len()
	}
}

impl<T> FusedIterator for Sorted<T> {}

impl<T> Iterator for Sorted<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.iter.size_hint()
	}

	fn last(mut self) -> Option<Self::Item> {
		self.next_back()
	}

	fn count(self) -> usize {
		self.iter.count()
	}

	fn fold<B, F>(self, init: B, f: F) -> B
	where
		F: FnMut(B, Self::Item) -> B,
	{
		self.iter.fold(init, f)
	}

	fn is_sorted(self) -> bool {
		true
	}
}

pub struct SortedBy<T, F> {
	pub(crate) iter: VecIntoIter<T>,
	sorter: Option<F>,
}

impl<T, F> SortedBy<T, F> {
	pub(crate) fn new(iter: impl IntoIterator<Item = T>, sorter: F) -> Self {
		Self {
			iter: iter.collect_to::<Vec<_>>().into_iter(),
			sorter: Some(sorter),
		}
	}

	const fn is_sorted(&self) -> bool {
		self.sorter.is_none()
	}
}

impl<T, F> SortedBy<T, F>
where
	F: FnMut(&T, &T) -> Ordering,
{
	unsafe fn sort_unchecked(&mut self) {
		let sorter = unsafe { self.sorter.take().unwrap_unchecked() };

		self.sort_with(sorter);
	}

	fn sort_with(&mut self, sorter: F) {
		self.iter = {
			let mut iter = mem::take(&mut self.iter).collect::<Vec<_>>();

			iter.sort_by(sorter);

			iter.into_iter()
		}
	}
}

impl<T, F> DoubleEndedIterator for SortedBy<T, F>
where
	F: FnMut(&T, &T) -> Ordering,
{
	fn next_back(&mut self) -> Option<Self::Item> {
		if !Self::is_sorted(self) {
			unsafe {
				self.sort_unchecked();
			}
		}

		self.iter.next_back()
	}
}

impl<T, F> ExactSizeIterator for SortedBy<T, F>
where
	F: FnMut(&T, &T) -> Ordering,
{
	fn len(&self) -> usize {
		self.iter.len()
	}
}

impl<T, F> FusedIterator for SortedBy<T, F> where F: FnMut(&T, &T) -> Ordering {}

impl<T, F> Iterator for SortedBy<T, F>
where
	F: FnMut(&T, &T) -> Ordering,
{
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		if !Self::is_sorted(self) {
			unsafe {
				self.sort_unchecked();
			}
		}

		self.iter.next()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.iter.size_hint()
	}

	fn count(self) -> usize {
		self.iter.count()
	}

	fn last(mut self) -> Option<Self::Item> {
		self.next_back()
	}
}

pub struct SortedByKey<T, K, F> {
	pub(crate) iter: VecIntoIter<T>,
	sorter: Option<F>,
	marker: PhantomData<K>,
}

impl<T, K, F> SortedByKey<T, K, F> {
	pub(crate) fn new(iter: impl IntoIterator<Item = T>, sorter: F) -> Self {
		Self {
			iter: iter.collect_to::<Vec<_>>().into_iter(),
			sorter: Some(sorter),
			marker: PhantomData,
		}
	}

	const fn is_sorted(&self) -> bool {
		self.sorter.is_none()
	}
}

impl<T, K: Ord, F> SortedByKey<T, K, F>
where
	F: FnMut(&T) -> K,
{
	unsafe fn sort_unchecked(&mut self) {
		let sorter = unsafe { self.sorter.take().unwrap_unchecked() };

		self.sort_with(sorter);
	}

	fn sort_with(&mut self, sorter: F) {
		self.iter = {
			let mut iter = mem::take(&mut self.iter).collect::<Vec<_>>();

			iter.sort_by_key(sorter);

			iter.into_iter()
		}
	}
}

impl<T, K: Ord, F> DoubleEndedIterator for SortedByKey<T, K, F>
where
	F: FnMut(&T) -> K,
{
	fn next_back(&mut self) -> Option<Self::Item> {
		if !Self::is_sorted(self) {
			unsafe {
				self.sort_unchecked();
			}
		}

		self.iter.next_back()
	}
}

impl<T, K: Ord, F> ExactSizeIterator for SortedByKey<T, K, F>
where
	F: FnMut(&T) -> K,
{
	fn len(&self) -> usize {
		self.iter.len()
	}
}

impl<T, K: Ord, F> FusedIterator for SortedByKey<T, K, F> where F: FnMut(&T) -> K {}

impl<T, K: Ord, F> Iterator for SortedByKey<T, K, F>
where
	F: FnMut(&T) -> K,
{
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		if !Self::is_sorted(self) {
			unsafe {
				self.sort_unchecked();
			}
		}

		self.iter.next()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.iter.size_hint()
	}

	fn count(self) -> usize {
		self.iter.count()
	}

	fn last(mut self) -> Option<Self::Item> {
		self.next_back()
	}
}

#[cfg(test)]
mod tests {
	use alloc::vec::Vec;

	use crate::IteratorExt;

	fn check_sorting<T: Ord>(v: &[T]) {
		assert!(v.is_sorted());
	}

	#[test]
	fn basic_sorting() {
		let v = [5u8, 2, 1, 7, 8, 3, 4, 9, 6];

		let sorted = v.into_iter().sorted().collect::<Vec<_>>();

		check_sorting(&sorted);
		assert_eq!(sorted, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
	}
}
