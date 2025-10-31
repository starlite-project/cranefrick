mod change;
mod set;

use core::{
	iter::{Copied, Enumerate, FusedIterator},
	slice,
};

pub use self::{change::*, set::*};
use super::{OffsetCellPrimitive, ValuedOffsetCellOptions};

#[must_use = "Iterators do nothing unless consumed"]
pub struct AffectManyCellsIter<'a, T: OffsetCellPrimitive> {
	iter: Enumerate<Copied<slice::Iter<'a, T>>>,
	start: i32,
}

impl<T: OffsetCellPrimitive> DoubleEndedIterator for AffectManyCellsIter<'_, T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter.next_back().map(|(index, value)| {
			ValuedOffsetCellOptions::new(value, self.start.wrapping_add_unsigned(index as u32))
		})
	}
}

impl<T: OffsetCellPrimitive> ExactSizeIterator for AffectManyCellsIter<'_, T> {
	fn len(&self) -> usize {
		self.iter.len()
	}
}

impl<T: OffsetCellPrimitive> FusedIterator for AffectManyCellsIter<'_, T> {}

impl<T: OffsetCellPrimitive> Iterator for AffectManyCellsIter<'_, T> {
	type Item = ValuedOffsetCellOptions<T>;

	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next().map(|(index, value)| {
			ValuedOffsetCellOptions::new(value, self.start.wrapping_add_unsigned(index as u32))
		})
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.iter.size_hint()
	}

	fn last(mut self) -> Option<Self::Item> {
		self.next_back()
	}
}
