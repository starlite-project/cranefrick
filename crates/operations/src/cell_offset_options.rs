use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellOffsetOptions {
	pub value: u8,
	pub offset: i32,
}

impl CellOffsetOptions {
	#[must_use]
	pub const fn new(value: u8, offset: i32) -> Self {
		Self { value, offset }
	}

	#[must_use]
	pub const fn value(self) -> u8 {
		self.value
	}

	#[must_use]
	pub const fn offset(self) -> i32 {
		self.offset
	}

	#[must_use]
	pub const fn into_parts(self) -> (u8, i32) {
		(self.value, self.offset)
	}
}
