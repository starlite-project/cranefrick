mod sealed;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MoveOptions<T: MoveOptionsInteger = u8> {
	value: T,
	offset: i32,
}

impl<T: MoveOptionsInteger> MoveOptions<T> {
	pub(crate) const fn new(value: T, offset: i32) -> Self {
		Self { value, offset }
	}

	#[must_use]
	pub const fn value(self) -> T {
		self.value
	}

	#[must_use]
	pub const fn offset(&self) -> i32 {
		self.offset
	}

	#[must_use]
	pub const fn into_parts(self) -> (T, i32) {
		(self.value, self.offset)
	}

	#[must_use]
	pub fn is_default(self) -> bool {
		self.value() == T::default() && matches!(self.offset(), 0)
	}
}

impl<T: MoveOptionsInteger> Clone for MoveOptions<T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: MoveOptionsInteger> Copy for MoveOptions<T> {}

impl<T: MoveOptionsInteger> Eq for MoveOptions<T> {}

impl<T: MoveOptionsInteger> PartialEq for MoveOptions<T> {
	fn eq(&self, other: &Self) -> bool {
		PartialEq::eq(&self.value, &other.value) && PartialEq::eq(&self.offset, &other.offset)
	}
}

pub trait MoveOptionsInteger: Copy + Default + Eq + self::sealed::Sealed {}

macro_rules! impl_move_options_integer {
	($($ty:ty)*) => {
		$(
			impl $crate::move_options::sealed::Sealed for $ty {}

			impl $crate::move_options::MoveOptionsInteger for $ty {}
		)*
	};
}

impl_move_options_integer!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize);
