use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

use super::ChangeCellOptions;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OutputOptions {
	Cell(ChangeCellOptions<i8>),
	Cells(Vec<ChangeCellOptions<i8>>),
	Char(u8),
	Str(Vec<u8>),
}

impl OutputOptions {
	#[must_use]
	pub const fn cell(value_offset: i8, offset: i32) -> Self {
		Self::Cell(ChangeCellOptions::new_value(value_offset, offset))
	}

	#[must_use]
	pub fn cells(values: impl IntoIterator<Item = ChangeCellOptions<i8>>) -> Self {
		Self::Cells(values.collect_to())
	}

	#[must_use]
	pub const fn char(value: u8) -> Self {
		Self::Char(value)
	}

	#[must_use]
	pub fn str(values: impl IntoIterator<Item = u8>) -> Self {
		Self::Str(values.collect_to())
	}
}
