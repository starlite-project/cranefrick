mod adapters;

#[cfg(feature = "alloc")]
use core::cmp::Ordering;

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
	fn sorted_by<F>(self, sorter: F) -> SortedBy<Self::Item, F>
	where
		Self: Sized,
		F: FnMut(&Self::Item, &Self::Item) -> Ordering,
	{
		SortedBy::new(self, sorter)
	}

	#[cfg(feature = "alloc")]
	fn sorted_by_key<K: Ord, F>(self, sorter: F) -> SortedByKey<Self::Item, K, F>
	where
		Self: Sized,
		F: FnMut(&Self::Item) -> K,
	{
		SortedByKey::new(self, sorter)
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
