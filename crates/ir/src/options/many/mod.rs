mod change;
mod set;

use core::{
	iter::{Copied, Enumerate, FusedIterator},
	slice,
};

pub use self::{change::*, set::*};

#[must_use = "Iterators do nothing unless consumed"]
pub struct AffectManyCellsIter<'a, T: Copy> {
	iter: Enumerate<Copied<slice::Iter<'a, T>>>,
	start: i32,
}

impl<T: Copy> DoubleEndedIterator for AffectManyCellsIter<'_, T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter
			.next_back()
			.map(|(index, value)| (value, self.start.wrapping_add_unsigned(index as u32)))
	}
}

impl<T: Copy> ExactSizeIterator for AffectManyCellsIter<'_, T> {
	fn len(&self) -> usize {
		self.iter.len()
	}
}

impl<T: Copy> FusedIterator for AffectManyCellsIter<'_, T> {}

impl<T: Copy> Iterator for AffectManyCellsIter<'_, T> {
	type Item = (T, i32);

	fn next(&mut self) -> Option<Self::Item> {
		self.iter
			.next()
			.map(|(index, value)| (value, self.start.wrapping_add_unsigned(index as u32)))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.iter.size_hint()
	}

	fn last(mut self) -> Option<Self::Item> {
		self.next_back()
	}
}
