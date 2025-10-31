use core::{iter::FusedIterator, ops::RangeInclusive};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetRangeOptions {
	value: u8,
	start: i32,
	end: i32,
}

impl SetRangeOptions {
	#[must_use]
	pub const fn new(value: u8, start: i32, end: i32) -> Self {
		Self { value, start, end }
	}

	#[must_use]
	pub const fn value(self) -> u8 {
		self.value
	}

	#[must_use]
	pub const fn start(self) -> i32 {
		self.start
	}

	#[must_use]
	pub const fn end(self) -> i32 {
		self.end
	}

	#[must_use]
	pub const fn range(self) -> RangeInclusive<i32> {
		self.start..=self.end
	}

	#[must_use]
	pub fn is_clobbering_cell(self) -> bool {
		self.range().contains(&0)
	}

	#[must_use]
	pub fn is_zeroing_cell(self) -> bool {
		self.is_clobbering_cell() && matches!(self.value(), 0)
	}

	#[must_use]
	pub const fn iter(self) -> SetRangeIter {
		SetRangeIter {
			range: self.range(),
			value: self.value,
		}
	}
}

impl IntoIterator for &SetRangeOptions {
	type IntoIter = SetRangeIter;
	type Item = (u8, i32);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl IntoIterator for SetRangeOptions {
	type IntoIter = SetRangeIter;
	type Item = (u8, i32);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub struct SetRangeIter {
	range: RangeInclusive<i32>,
	value: u8,
}

impl DoubleEndedIterator for SetRangeIter {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.range.next_back().map(|index| (self.value, index))
	}
}

impl FusedIterator for SetRangeIter {}

impl Iterator for SetRangeIter {
	type Item = (u8, i32);

	fn next(&mut self) -> Option<Self::Item> {
		self.range.next().map(|index| (self.value, index))
	}
}
