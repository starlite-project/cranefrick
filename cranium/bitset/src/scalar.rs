use core::{
	iter::FusedIterator,
	mem,
	ops::{Add, BitAnd, BitOr, Not, Shl, Shr, Sub},
};

use cranefrick_utils::UnwrapFrom as _;
use serde::{Deserialize, Serialize};
#[cfg(feature = "alloc")]
use {
	alloc::string::ToString as _,
	core::fmt::{Debug, Formatter, Result as FmtResult},
};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ScalarBitSet<T>(pub T);

impl<T: ScalarBitSetStorage> ScalarBitSet<T> {
	#[must_use]
	pub fn new() -> Self {
		Self(T::from(0))
	}

	#[must_use]
	pub fn from_range(lo: u8, hi: u8) -> Self {
		assert!(lo <= hi);
		assert!(hi <= Self::capacity());

		let one = T::from(1);

		let hi_range = if hi >= 1 {
			(one << (hi - 1)) + ((one << (hi - 1)) - one)
		} else {
			T::from(0)
		};

		let lo_range = (one << lo) - one;

		Self(hi_range - lo_range)
	}

	#[must_use]
	pub fn capacity() -> u8 {
		u8::unwrap_from(mem::size_of::<T>()) * 8
	}

	pub fn len(&self) -> u8 {
		self.0.count_ones()
	}

	pub fn is_empty(&self) -> bool {
		self.0 == T::from(0)
	}

	pub fn contains(&self, i: u8) -> bool {
		assert!(i < Self::capacity());
		self.0 & (T::from(1) << i) != T::from(0)
	}

	pub fn insert(&mut self, i: u8) -> bool {
		let is_new = !self.contains(i);
		self.0 = self.0 | (T::from(1) << i);
		is_new
	}

	pub fn remove(&mut self, i: u8) -> bool {
		let was_present = self.contains(i);
		self.0 = self.0 & !(T::from(1) << i);
		was_present
	}

	pub fn clear(&mut self) {
		self.0 = T::from(0);
	}

	pub fn pop_min(&mut self) -> Option<u8> {
		let min = self.min()?;
		self.remove(min);
		Some(min)
	}

	pub fn pop_max(&mut self) -> Option<u8> {
		let max = self.max()?;
		self.remove(max);
		Some(max)
	}

	pub fn min(&self) -> Option<u8> {
		if self.0 == T::from(0) {
			None
		} else {
			Some(self.0.trailing_zeros())
		}
	}

	pub fn max(&self) -> Option<u8> {
		if self.0 == T::from(0) {
			None
		} else {
			let leading_zeros = self.0.leading_zeros();
			Some(Self::capacity() - leading_zeros - 1)
		}
	}

	pub const fn iter(self) -> ScalarIter<T> {
		ScalarIter { inner: self }
	}
}

#[cfg(feature = "arbitrary")]
impl<'a, T> arbitrary::Arbitrary<'a> for ScalarBitSet<T>
where
	T: arbitrary::Arbitrary<'a>,
{
	fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
		T::arbitrary(u).map(Self)
	}

	fn arbitrary_take_rest(u: arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
		T::arbitrary_take_rest(u).map(Self)
	}

	fn size_hint(depth: usize) -> (usize, Option<usize>) {
		T::size_hint(depth)
	}

	fn try_size_hint(
		depth: usize,
	) -> arbitrary::Result<(usize, Option<usize>), arbitrary::MaxRecursionReached> {
		T::try_size_hint(depth)
	}
}

#[cfg(feature = "alloc")]
impl<T: ScalarBitSetStorage> Debug for ScalarBitSet<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let mut s = f.debug_struct("ScalarBitSet");
		for i in 0..Self::capacity() {
			s.field(&i.to_string(), &self.contains(i));
		}

		Ok(())
	}
}

impl<T: ScalarBitSetStorage> Default for ScalarBitSet<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T: ScalarBitSetStorage> IntoIterator for ScalarBitSet<T> {
	type IntoIter = ScalarIter<T>;
	type Item = u8;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

#[repr(transparent)]
pub struct ScalarIter<T> {
	inner: ScalarBitSet<T>,
}

impl<T: ScalarBitSetStorage> DoubleEndedIterator for ScalarIter<T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.inner.pop_max()
	}
}

impl<T: ScalarBitSetStorage> ExactSizeIterator for ScalarIter<T> {
	fn len(&self) -> usize {
		self.inner.len().into()
	}
}

impl<T: ScalarBitSetStorage> FusedIterator for ScalarIter<T> {}

impl<T: ScalarBitSetStorage> Iterator for ScalarIter<T> {
	type Item = u8;

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.pop_min()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let size = self.inner.len().into();

		(size, Some(size))
	}

	fn last(mut self) -> Option<Self::Item> {
		self.next_back()
	}

	fn count(self) -> usize {
		self.len()
	}

	fn is_sorted(self) -> bool {
		true
	}
}

pub trait ScalarBitSetStorage:
	Add<Output = Self>
	+ BitAnd<Output = Self>
	+ BitOr<Output = Self>
	+ Copy
	+ Default
	+ From<u8>
	+ Not<Output = Self>
	+ PartialEq
	+ Shl<u8, Output = Self>
	+ Shr<u8, Output = Self>
	+ Sub<Output = Self>
{
	fn leading_zeros(self) -> u8;

	fn trailing_zeros(self) -> u8;

	fn count_ones(self) -> u8;
}

macro_rules! impl_storage {
	($($ty:ty)*) => {
		$(
			impl $crate::scalar::ScalarBitSetStorage for $ty {
				fn leading_zeros(self) -> u8 {
					use ::cranefrick_utils::UnwrapFrom as _;

					u8::unwrap_from(self.leading_zeros())
				}

				fn trailing_zeros(self) -> u8 {
					use ::cranefrick_utils::UnwrapFrom as _;

					u8::unwrap_from(self.trailing_zeros())
				}

				fn count_ones(self) -> u8 {
					use ::cranefrick_utils::UnwrapFrom as _;

					u8::unwrap_from(self.count_ones())
				}
			}
		)*
	};
}

impl_storage!(u8 u16 u32 u64 u128 usize);

#[cfg(test)]
mod tests {
	use core::iter;

	use super::{ScalarBitSet, ScalarBitSetStorage};

	#[test]
	fn contains() {
		fn assert_contains<T: ScalarBitSetStorage>(
			s: ScalarBitSet<T>,
			iter: impl IntoIterator<Item = u8>,
		) {
			for i in iter {
				assert!(s.contains(i));
			}
		}

		fn assert_not_contains<T: ScalarBitSetStorage>(
			s: ScalarBitSet<T>,
			iter: impl IntoIterator<Item = u8>,
		) {
			for i in iter {
				assert!(!s.contains(i));
			}
		}

		let s = ScalarBitSet(255u8);

		assert_contains(s, 0..7);

		let s = ScalarBitSet(0u8);
		assert_not_contains(s, 0..7);

		let s = ScalarBitSet(127u8);
		assert_contains(s, 0..6);
		assert_not_contains(s, iter::once(7));

		let s = ScalarBitSet(2u8 | 4 | 64);
		assert_contains(s, [1, 2, 6]);
		assert_not_contains(s, [0, 3, 4, 5, 7]);

		let s = ScalarBitSet(4u16 | 8 | 256 | 1024);
		assert_contains(s, [2, 3, 8, 10]);
		assert_not_contains(s, [0, 1, 4, 5, 6, 7, 9, 11]);
	}

	#[test]
	fn minmax() {
		let s = ScalarBitSet(u8::MAX);
		assert_eq!((s.min(), s.max()), (Some(0), Some(7)));

		let s = ScalarBitSet(u8::MIN);
		assert_eq!((s.min(), s.max()), (None, None));

		let s = ScalarBitSet(127u8);
		assert_eq!((s.min(), s.max()), (Some(0), Some(6)));

		let s = ScalarBitSet(2u8 | 4 | 64);
		assert_eq!((s.min(), s.max()), (Some(1), Some(6)));

		let s = ScalarBitSet(4u16 | 8 | 256 | 1024);
		assert_eq!((s.min(), s.max()), (Some(2), Some(10)));
	}

	#[test]
	fn from_range() {
		let s = ScalarBitSet::<u8>::from_range(5, 5);
		assert_eq!(s.0, 0);

		let s = ScalarBitSet::<u8>::from_range(0, 8);
		assert_eq!(s.0, 255);

		let s = ScalarBitSet::<u16>::from_range(0, 8);
		assert_eq!(s.0, 255);

		let s = ScalarBitSet::<u16>::from_range(0, 16);
		assert_eq!(s.0, 65535);

		let s = ScalarBitSet::<u8>::from_range(5, 6);
		assert_eq!(s.0, 32);

		let s = ScalarBitSet::<u8>::from_range(3, 7);
		assert_eq!(s.0, 120); // 8 | 16 | 32 | 64

		let s = ScalarBitSet::<u16>::from_range(5, 11);
		assert_eq!(s.0, 2016); // 32 | 64 | 128 | 256 | 512 | 1024
	}
}
