use std::{
	iter::{Copied, Enumerate, FusedIterator},
	num::NonZero,
	ops::Range,
	slice,
};

use frick_utils::{GetOrZero, IntoIteratorExt as _};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetManyCellsOptions {
	pub values: Vec<u8>,
	pub start: Option<NonZero<i32>>,
}

impl SetManyCellsOptions {
	pub fn new(values: impl IntoIterator<Item = u8>, start: i32) -> Self {
		Self {
			values: values.collect_to(),
			start: NonZero::new(start),
		}
	}

	#[must_use]
	pub fn range(&self) -> Range<i32> {
		let start = self.start.get_or_zero();
		let end = start.wrapping_add_unsigned(self.values.len() as u32);

		start..end
	}

	#[must_use]
	pub fn value_at(&self, offset: i32) -> Option<u8> {
		let mut range = self.range();

		let index = range.position(|x| x == offset)?;

		self.values.get(index).copied()
	}

	#[must_use]
	pub fn is_zeroing_cell(&self) -> bool {
		matches!(self.value_at(0), Some(0))
	}

	#[must_use]
	pub fn iter(&self) -> SetManyCellsIter<'_> {
		SetManyCellsIter {
			iter: self.values.iter().copied().enumerate(),
			start: self.start.get_or_zero(),
		}
	}
}

impl<'a> IntoIterator for &'a SetManyCellsOptions {
	type IntoIter = SetManyCellsIter<'a>;
	type Item = (u8, i32);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub struct SetManyCellsIter<'a> {
	iter: Enumerate<Copied<slice::Iter<'a, u8>>>,
	start: i32,
}

impl DoubleEndedIterator for SetManyCellsIter<'_> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter
			.next_back()
			.map(|(index, value)| (value, self.start.wrapping_add_unsigned(index as u32)))
	}
}

impl ExactSizeIterator for SetManyCellsIter<'_> {
	fn len(&self) -> usize {
		self.iter.len()
	}
}

impl FusedIterator for SetManyCellsIter<'_> {}

impl Iterator for SetManyCellsIter<'_> {
	type Item = (u8, i32);

	fn next(&mut self) -> Option<Self::Item> {
		self.iter
			.next()
			.map(|(index, value)| (value, self.start.wrapping_add_unsigned(index as u32)))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.iter.size_hint()
	}
}
