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
	fn sorted_by<F>(self, sorter: F) -> SortedBy<Self::Item, F>
	where
		Self: Sized,
		F: FnMut(&Self::Item, &Self::Item) -> Ordering,
	{
		SortedBy::new(self, sorter)
	}

	#[cfg(feature = "alloc")]
	fn sorted_unstable_by<F>(self, sorter: F) -> SortedUnstableBy<Self::Item, F>
	where
		Self: Sized,
		F: FnMut(&Self::Item, &Self::Item) -> Ordering,
	{
		SortedUnstableBy::new(self, sorter)
	}

	#[cfg(feature = "alloc")]
	fn sorted_by_key<K: Ord, F>(self, sorter: F) -> SortedByKey<Self::Item, K, F>
	where
		Self: Sized,
		F: FnMut(&Self::Item) -> K,
	{
		SortedByKey::new(self, sorter)
	}

	#[cfg(feature = "alloc")]
	fn sorted_unstable_by_key<K: Ord, F>(self, sorter: F) -> SortedUnstableByKey<Self::Item, K, F>
	where
		Self: Sized,
		F: FnMut(&Self::Item) -> K,
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
