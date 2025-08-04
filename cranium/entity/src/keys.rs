use core::{iter::FusedIterator, marker::PhantomData};

use super::EntityRef;

pub struct Keys<K: EntityRef> {
	start: usize,
	end: usize,
	marker: PhantomData<K>,
}

impl<K: EntityRef> Keys<K> {
	#[must_use]
	pub const fn new(len: usize) -> Self {
		Self {
			start: 0,
			end: len,
			marker: PhantomData,
		}
	}
}

impl<K: EntityRef> DoubleEndedIterator for Keys<K> {
	fn next_back(&mut self) -> Option<Self::Item> {
		if self.end > self.start {
			let k = K::new(self.end - 1);
			self.end -= 1;
			Some(k)
		} else {
			None
		}
	}
}

impl<K: EntityRef> ExactSizeIterator for Keys<K> {
	fn len(&self) -> usize {
		self.end - self.start
	}
}

impl<K: EntityRef> FusedIterator for Keys<K> {}

impl<K: EntityRef> Iterator for Keys<K> {
	type Item = K;

	fn next(&mut self) -> Option<Self::Item> {
		if self.start < self.end {
			let k = K::new(self.start);
			self.start += 1;
			Some(k)
		} else {
			None
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let size = self.end - self.start;
		(size, Some(size))
	}
}
