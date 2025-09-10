use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateCellData {
	factor: i8,
	offset: i32,
}

impl DuplicateCellData {
	pub(crate) const fn new(factor: i8, offset: i32) -> Self {
		Self { factor, offset }
	}

	#[must_use]
	pub const fn factor(self) -> i8 {
		self.factor
	}

	#[must_use]
	pub const fn offset(self) -> i32 {
		self.offset
	}

	#[must_use]
	pub const fn into_parts(self) -> (i8, i32) {
		(self.factor(), self.offset())
	}
}
