use alloc::vec::Vec;
use core::{
	fmt::{Debug, Formatter, Result as FmtResult},
	ops::Range,
};

use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

use super::AffectManyCellsIter;
use crate::ValuedOffsetCellOptions;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
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
	pub const fn end(&self) -> i32 {
		let start = self.start();

		start.wrapping_add_unsigned(self.values.len() as u32)
	}

	#[must_use]
	pub const fn range(&self) -> Range<i32> {
		self.start()..self.end()
	}

	#[must_use]
	pub fn value_at(&self, offset: i32) -> Option<i8> {
		let mut range = self.range();

		let index = range.position(|x| x == offset)?;

		self.values.get(index).copied()
	}

	pub fn value_at_mut(&mut self, offset: i32) -> Option<&mut i8> {
		let mut range = self.range();

		let index = range.position(|x| x == offset)?;

		self.values.get_mut(index)
	}

	pub fn set_value_at(&mut self, offset: i32, value: i8) -> bool {
		if let Some(current_value) = self.value_at_mut(offset) {
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

impl Debug for ChangeManyCellsOptions {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let mut s = f.debug_tuple("ChangeManyCellsOptions");

		for offset_options in self {
			s.field(&offset_options);
		}

		s.finish()
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
