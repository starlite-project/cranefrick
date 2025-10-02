use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetRangeOptions {
	pub value: u8,
	pub start: i32,
	pub end: i32,
}

impl SetRangeOptions {
	#[must_use]
	pub const fn new(value: u8, start: i32, end: i32) -> Self {
		Self { value, start, end }
	}

	#[must_use]
	pub const fn range(self) -> RangeInclusive<i32> {
		self.start..=self.end
	}

	#[must_use]
	pub fn is_zeroing_cell(self) -> bool {
		matches!(self.value, 0) && self.range().contains(&0)
	}
}
