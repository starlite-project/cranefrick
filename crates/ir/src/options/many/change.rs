use alloc::vec::Vec;
use core::ops::Range;

use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

use super::AffectManyCellsIter;
use crate::ValuedOffsetCellOptions;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeManyCellsOptions {
	values: Vec<i8>,
	start: i32,
}

impl ChangeManyCellsOptions {
	pub fn new(values: impl IntoIterator<Item = i8>, start: i32) -> Self {
		Self {
			values: values.collect_to(),
			start,
		}
	}

	#[must_use]
	pub const fn values(&self) -> &[i8] {
		self.values.as_slice()
	}

	#[must_use]
	pub const fn start(&self) -> i32 {
		self.start
	}

	#[must_use]
	pub const fn range(&self) -> Range<i32> {
		let start = self.start;
		let end = start.wrapping_add_unsigned(self.values.len() as u32);

		start..end
	}

	#[must_use]
	pub fn value_at(&self, offset: i32) -> Option<i8> {
		let mut range = self.range();

		let index = range.position(|x| x == offset)?;

		self.values.get(index).copied()
	}

	pub fn set_value_at(&mut self, offset: i32, value: i8) -> bool {
		let mut range = self.range();

		if let Some(current_value) = range
			.position(|x| x == offset)
			.and_then(|index| self.values.get_mut(index))
		{
			*current_value = value;
			true
		} else {
			false
		}
	}

	pub fn iter(&self) -> ChangeManyCellsIter<'_> {
		ChangeManyCellsIter {
			iter: self.values.iter().copied().enumerate(),
			start: self.start,
		}
	}
}

impl<'a> IntoIterator for &'a ChangeManyCellsOptions {
	type IntoIter = ChangeManyCellsIter<'a>;
	type Item = ValuedOffsetCellOptions<i8>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub type ChangeManyCellsIter<'a> = AffectManyCellsIter<'a, i8>;
