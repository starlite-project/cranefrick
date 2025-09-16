use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveOptions<T = u8> {
	factor: T,
	offset: i32,
}

impl<T> MoveOptions<T> {
	pub(crate) const fn new(factor: T, offset: i32) -> Self {
		Self { factor, offset }
	}

	#[must_use]
	pub const fn factor_ref(&self) -> &T {
		&self.factor
	}

	#[must_use]
	pub const fn offset(&self) -> i32 {
		self.offset
	}

	#[must_use]
	pub fn into_parts(self) -> (T, i32) {
		(self.factor, self.offset)
	}
}

impl<T: Copy> MoveOptions<T> {
	#[must_use]
	pub const fn factor(self) -> T {
		self.factor
	}
}
