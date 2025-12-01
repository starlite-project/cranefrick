mod adapters;

#[cfg(feature = "alloc")]
use core::cmp::Ordering;
use core::iter::{self, Chain};

#[cfg(feature = "alloc")]
pub use self::adapters::*;

pub trait IteratorExt: Iterator {
	#[cfg(feature = "alloc")]
	fn sorted(self) -> Sorted<Self::Item>
	where
		Self: Sized,
		Self::Item: Ord,
	{
		Sorted::new(self)
	}

	#[cfg(feature = "alloc")]
	fn sorted_unstable(self) -> SortedUnstable<Self::Item>
	where
		Self: Sized,
		Self::Item: Ord,
	{
		SortedUnstable::new(self)
	}

	#[cfg(feature = "alloc")]
	fn sorted_by(
		self,
		sorter: impl FnMut(&Self::Item, &Self::Item) -> Ordering,
	) -> SortedBy<Self::Item>
	where
		Self: Sized,
	{
		SortedBy::new(self, sorter)
	}

	#[cfg(feature = "alloc")]
	fn sorted_unstable_by(
		self,
		sorter: impl FnMut(&Self::Item, &Self::Item) -> Ordering,
	) -> SortedUnstableBy<Self::Item>
	where
		Self: Sized,
	{
		SortedUnstableBy::new(self, sorter)
	}

	#[cfg(feature = "alloc")]
	fn sorted_by_key<K: Ord>(
		self,
		sorter: impl FnMut(&Self::Item) -> K,
	) -> SortedByKey<Self::Item, K>
	where
		Self: Sized,
	{
		SortedByKey::new(self, sorter)
	}

	#[cfg(feature = "alloc")]
	fn sorted_unstable_by_key<K: Ord>(
		self,
		sorter: impl FnMut(&Self::Item) -> K,
	) -> SortedUnstableByKey<Self::Item, K>
	where
		Self: Sized,
	{
		SortedUnstableByKey::new(self, sorter)
	}

	fn chain_once(self, item: Self::Item) -> Chain<Self, iter::Once<Self::Item>>
	where
		Self: Sized,
	{
		self.chain(iter::once(item))
	}
}

impl<T: Iterator> IteratorExt for T {}

pub trait IntoIteratorExt: IntoIterator {
	fn collect_to<B>(self) -> B
	where
		Self: Sized,
		B: FromIterator<Self::Item>,
	{
		Iterator::collect(self.into_iter())
	}
}

impl<T: IntoIterator> IntoIteratorExt for T {}
