use core::{iter::FusedIterator, marker::PhantomData, ops::Range};

use super::EntityRef;

#[repr(transparent)]
pub struct EntityRangeIter<E> {
	inner: Range<usize>,
	marker: PhantomData<E>,
}

impl<E: EntityRef> DoubleEndedIterator for EntityRangeIter<E> {
	fn next_back(&mut self) -> Option<Self::Item> {
		let i = self.inner.next_back()?;
		Some(E::new(i))
	}
}

impl<E: EntityRef> ExactSizeIterator for EntityRangeIter<E> {
	fn len(&self) -> usize {
		self.inner.len()
	}
}

impl<E: EntityRef> From<Range<usize>> for EntityRangeIter<E> {
	fn from(value: Range<usize>) -> Self {
		Self {
			inner: value,
			marker: PhantomData,
		}
	}
}

impl<E: EntityRef> From<Range<E>> for EntityRangeIter<E> {
	fn from(value: Range<E>) -> Self {
		Self::from(value.start.index()..value.end.index())
	}
}

impl<E: EntityRef> FusedIterator for EntityRangeIter<E> {}

impl<E: EntityRef> Iterator for EntityRangeIter<E> {
	type Item = E;

	fn next(&mut self) -> Option<Self::Item> {
		let i = self.inner.next()?;
		Some(E::new(i))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	fn last(mut self) -> Option<Self::Item> {
		self.next_back()
	}
}
