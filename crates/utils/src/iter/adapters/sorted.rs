use alloc::vec::{IntoIter as VecIntoIter, Vec};
use core::{cmp::Ordering, iter::FusedIterator, marker::PhantomData};

use crate::IntoIteratorExt as _;

#[repr(transparent)]
pub struct Sorted<T> {
	pub(crate) iter: VecIntoIter<T>,
}

impl<T: Ord> Sorted<T> {
	pub(crate) fn new(iter: impl IntoIterator<Item = T>) -> Self {
		let mut iter = iter.collect_to::<Vec<_>>().into_iter();

		iter.as_mut_slice().sort();

		Self { iter }
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

	fn is_sorted(self) -> bool {
		true
	}
}

#[repr(transparent)]
pub struct SortedBy<T> {
	pub(crate) iter: VecIntoIter<T>,
}

impl<T> SortedBy<T> {
	pub(crate) fn new(
		iter: impl IntoIterator<Item = T>,
		sorter: impl FnMut(&T, &T) -> Ordering,
	) -> Self {
		let mut iter = iter.collect_to::<Vec<_>>().into_iter();

		iter.as_mut_slice().sort_by(sorter);

		Self { iter }
	}
}

impl<T> DoubleEndedIterator for SortedBy<T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter.next_back()
	}
}

impl<T> ExactSizeIterator for SortedBy<T> {
	fn len(&self) -> usize {
		self.iter.len()
	}
}

impl<T> FusedIterator for SortedBy<T> {}

impl<T> Iterator for SortedBy<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
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

#[repr(transparent)]
pub struct SortedByKey<T> {
	pub(crate) iter: VecIntoIter<T>,
}

impl<T> SortedByKey<T> {
	pub(crate) fn new<K: Ord>(
		iter: impl IntoIterator<Item = T>,
		sorter: impl FnMut(&T) -> K,
	) -> Self {
		let mut iter = iter.collect_to::<Vec<_>>().into_iter();

		iter.as_mut_slice().sort_by_key(sorter);

		Self { iter }
	}
}

impl<T> DoubleEndedIterator for SortedByKey<T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter.next_back()
	}
}

impl<T> ExactSizeIterator for SortedByKey<T> {
	fn len(&self) -> usize {
		self.iter.len()
	}
}

impl<T> FusedIterator for SortedByKey<T> {}

impl<T> Iterator for SortedByKey<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
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

#[repr(transparent)]
pub struct SortedUnstable<T> {
	pub(crate) iter: VecIntoIter<T>,
}

impl<T: Ord> SortedUnstable<T> {
	pub(crate) fn new(iter: impl IntoIterator<Item = T>) -> Self {
		let mut iter = iter.collect_to::<Vec<_>>().into_iter();

		iter.as_mut_slice().sort_unstable();

		Self { iter }
	}
}

impl<T> DoubleEndedIterator for SortedUnstable<T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter.next_back()
	}
}

impl<T> ExactSizeIterator for SortedUnstable<T> {
	fn len(&self) -> usize {
		self.iter.len()
	}
}

impl<T> FusedIterator for SortedUnstable<T> {}

impl<T> Iterator for SortedUnstable<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
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

	fn is_sorted(self) -> bool {
		true
	}
}

pub struct SortedUnstableBy<T> {
	pub(crate) iter: VecIntoIter<T>,
}

impl<T> SortedUnstableBy<T> {
	pub(crate) fn new(
		iter: impl IntoIterator<Item = T>,
		sorter: impl FnMut(&T, &T) -> Ordering,
	) -> Self {
		let mut iter = iter.collect_to::<Vec<_>>().into_iter();

		iter.as_mut_slice().sort_unstable_by(sorter);

		Self { iter }
	}
}

impl<T> DoubleEndedIterator for SortedUnstableBy<T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter.next_back()
	}
}

impl<T> ExactSizeIterator for SortedUnstableBy<T> {
	fn len(&self) -> usize {
		self.iter.len()
	}
}

impl<T> FusedIterator for SortedUnstableBy<T> {}

impl<T> Iterator for SortedUnstableBy<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
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

#[repr(transparent)]
pub struct SortedUnstableByKey<T> {
	pub(crate) iter: VecIntoIter<T>,
}

impl<T> SortedUnstableByKey<T> {
	pub(crate) fn new<K: Ord>(
		iter: impl IntoIterator<Item = T>,
		sorter: impl FnMut(&T) -> K,
	) -> Self {
		let mut iter = iter.collect_to::<Vec<_>>().into_iter();

		iter.as_mut_slice().sort_unstable_by_key(sorter);

		Self { iter }
	}
}

impl<T> DoubleEndedIterator for SortedUnstableByKey<T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter.next_back()
	}
}

impl<T> ExactSizeIterator for SortedUnstableByKey<T> {
	fn len(&self) -> usize {
		self.iter.len()
	}
}

impl<T> FusedIterator for SortedUnstableByKey<T> {}

impl<T> Iterator for SortedUnstableByKey<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
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
