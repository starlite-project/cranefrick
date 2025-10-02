use std::{num::NonZero, ops::Range};

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
}
