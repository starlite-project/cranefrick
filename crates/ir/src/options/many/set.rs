use alloc::vec::Vec;
use core::{
	fmt::{Debug, Formatter, Result as FmtResult},
	ops::Range,
};

use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

use super::AffectManyCellsIter;
use crate::{SetRangeOptions, ValuedOffsetCellOptions};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetManyCellsOptions {
	values: Vec<u8>,
	start: i32,
}

impl SetManyCellsOptions {
	pub fn new(values: impl IntoIterator<Item = u8>, start: i32) -> Self {
		Self {
			values: values.collect_to(),
			start,
		}
	}

	#[must_use]
	pub const fn values(&self) -> &[u8] {
		self.values.as_slice()
	}

	#[must_use]
	pub const fn start(&self) -> i32 {
		self.start
	}

	#[must_use]
	pub const fn end(&self) -> i32 {
		let start = self.start();

		start.wrapping_add_unsigned(self.values.len() as u32)
	}

	#[must_use]
	pub const fn range(&self) -> Range<i32> {
		self.start()..self.end()
	}

	#[must_use]
	pub fn is_clobbering_cell(&self) -> bool {
		self.value_at(0).is_some()
	}

	#[must_use]
	pub fn is_zeroing_cell(&self) -> bool {
		matches!(self.value_at(0), Some(0))
	}

	#[must_use]
	pub fn value_at(&self, offset: i32) -> Option<u8> {
		let mut range = self.range();

		let index = range.position(|x| x == offset)?;

		self.values.get(index).copied()
	}

	pub fn set_value_at(&mut self, offset: i32, value: u8) -> bool {
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

	pub fn iter(&self) -> SetManyCellsIter<'_> {
		SetManyCellsIter {
			iter: self.values.iter().copied().enumerate(),
			start: self.start,
		}
	}
}

impl Debug for SetManyCellsOptions {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let mut s = f.debug_tuple("SetManyCellsOptions");

		for offset_option in self {
			s.field(&offset_option);
		}

		s.finish()
	}
}

impl From<SetRangeOptions> for SetManyCellsOptions {
	fn from(value: SetRangeOptions) -> Self {
		let range = value.range();

		let values = range.map(|_| value.value());

		Self::new(values, value.start())
	}
}

impl<'a> IntoIterator for &'a SetManyCellsOptions {
	type IntoIter = SetManyCellsIter<'a>;
	type Item = ValuedOffsetCellOptions<u8>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub type SetManyCellsIter<'a> = AffectManyCellsIter<'a, u8>;
