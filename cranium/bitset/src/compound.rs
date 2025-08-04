use alloc::boxed::Box;
use core::{cmp, iter, mem};

use cranefrick_utils::UnwrapFrom as _;
use serde::{Deserialize, Serialize};

use super::{ScalarBitSet, ScalarBitSetStorage, ScalarIter};

#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompoundBitSet<T = usize> {
	values: Box<[ScalarBitSet<T>]>,
	max: Option<u32>,
}

impl<T: ScalarBitSetStorage> CompoundBitSet<T> {
	const BITS_PER_SCALAR: usize = mem::size_of::<T>() * 8;

	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self {
		let mut bitset = Self::new();
		bitset.ensure_capacity(capacity);
		bitset
	}

	#[must_use]
	pub fn len(&self) -> usize {
		self.values.iter().map(|sub| usize::from(sub.len())).sum()
	}

	#[must_use]
	pub fn capacity(&self) -> usize {
		self.values.len() * Self::BITS_PER_SCALAR
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		matches!(self.len(), 0)
	}

	#[must_use]
	pub fn contains(&self, i: usize) -> bool {
		let (word, bit) = Self::word_and_bit(i);
		self.values.get(word).is_some_and(|set| set.contains(bit))
	}

	pub fn ensure_capacity(&mut self, n: usize) {
		let Some(n) = n.checked_sub(1) else {
			return;
		};

		let (word, ..) = Self::word_and_bit(n);

		if word >= self.values.len() {
			assert!(word < usize::unwrap_from(isize::MAX));

			let delta = word - self.values.len();
			let to_grow = delta + 1;

			let to_grow = cmp::max(to_grow, self.values.len() * 2);
			let to_grow = cmp::max(to_grow, 4);

			let new_values = self
				.values
				.iter()
				.copied()
				.chain(iter::repeat_n(ScalarBitSet::new(), to_grow))
				.collect();

			self.values = new_values;
		}
	}

	pub fn insert(&mut self, i: usize) -> bool {
		self.ensure_capacity(i + 1);

		let (word, bit) = Self::word_and_bit(i);
		let is_new = self.values[word].insert(bit);

		let i = u32::unwrap_from(i);
		self.max = self.max.map(|max| cmp::max(max, i)).or(Some(i));

		is_new
	}

	pub fn remove(&mut self, i: usize) -> bool {
		let (word, bit) = Self::word_and_bit(i);
		if word < self.values.len() {
			let sub = &mut self.values[word];
			let was_present = sub.remove(bit);
			if was_present && self.max() == Some(i) {
				self.update_max(word);
			}

			was_present
		} else {
			false
		}
	}

	pub fn max(&self) -> Option<usize> {
		self.max.map(usize::unwrap_from)
	}

	pub fn pop(&mut self) -> Option<usize> {
		let max = self.max()?;
		self.remove(max);
		Some(max)
	}

	pub fn clear(&mut self) {
		let Some(max) = self.max() else {
			return;
		};

		let (word, ..) = Self::word_and_bit(max);
		debug_assert!(self.values[word + 1..].iter().all(ScalarBitSet::is_empty));

		for sub in &mut self.values[..=word] {
			*sub = ScalarBitSet::new();
		}

		self.max = None;
	}

	#[must_use]
	pub const fn iter(&self) -> CompoundIter<'_, T> {
		CompoundIter {
			inner: self,
			word: 0,
			sub: None,
		}
	}

	pub fn scalars(&self) -> impl Iterator<Item = ScalarBitSet<T>> + '_ {
		let nwords = self
			.max
			.map_or(0, |n| 1 + (n as usize / Self::BITS_PER_SCALAR));

		self.values.iter().copied().take(nwords)
	}

	fn word_and_bit(i: usize) -> (usize, u8) {
		let word = i / Self::BITS_PER_SCALAR;
		let bit = i % Self::BITS_PER_SCALAR;
		let bit = u8::unwrap_from(bit);
		(word, bit)
	}

	const fn value(word: usize, bit: u8) -> usize {
		let bit = bit as usize;
		debug_assert!(bit < Self::BITS_PER_SCALAR);
		word * Self::BITS_PER_SCALAR + bit
	}

	fn update_max(&mut self, word_of_old_max: usize) {
		self.max = self.values[0..word_of_old_max + 1]
			.iter()
			.enumerate()
			.rev()
			.find_map(|(word, sub)| {
				let bit = sub.max()?;
				u32::try_from(Self::value(word, bit)).ok()
			});
	}
}

impl<'a, T: ScalarBitSetStorage> IntoIterator for &'a CompoundBitSet<T> {
	type IntoIter = CompoundIter<'a, T>;
	type Item = usize;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub struct CompoundIter<'a, T = usize> {
	inner: &'a CompoundBitSet<T>,
	word: usize,
	sub: Option<ScalarIter<T>>,
}

impl<T: ScalarBitSetStorage> ExactSizeIterator for CompoundIter<'_, T> {
	fn len(&self) -> usize {
		self.inner.len()
	}
}

impl<T: ScalarBitSetStorage> Iterator for CompoundIter<'_, T> {
	type Item = usize;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			if let Some(sub) = &mut self.sub {
				if let Some(bit) = sub.next() {
					return Some(CompoundBitSet::<T>::value(self.word, bit));
				}

				self.word += 1;
			}

			self.sub = Some(self.inner.values.get(self.word)?.iter());
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let size = self.inner.len();

		(size, Some(size))
	}

	fn count(self) -> usize {
		self.len()
	}
}

#[cfg(test)]
mod tests {
	use super::CompoundBitSet;

	#[test]
	fn zero_capacity_does_not_alloc() {
		let set = CompoundBitSet::<u32>::with_capacity(0);
		assert_eq!(set.capacity(), 0);
		let set = CompoundBitSet::<usize>::new();
		assert_eq!(set.capacity(), 0);
	}
}
