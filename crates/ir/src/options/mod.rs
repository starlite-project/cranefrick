mod sealed;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CellChangeOptions<T: CellChangeOptionsInteger = u8> {
	value: T,
	offset: i32,
}

impl<T: CellChangeOptionsInteger> CellChangeOptions<T> {
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

impl<T: CellChangeOptionsInteger> Clone for CellChangeOptions<T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: CellChangeOptionsInteger> Copy for CellChangeOptions<T> {}

impl<T: CellChangeOptionsInteger> Eq for CellChangeOptions<T> {}

impl<T: CellChangeOptionsInteger> PartialEq for CellChangeOptions<T> {
	fn eq(&self, other: &Self) -> bool {
		PartialEq::eq(&self.value, &other.value) && PartialEq::eq(&self.offset, &other.offset)
	}
}

pub trait CellChangeOptionsInteger: Copy + Default + Eq + self::sealed::Sealed {}

macro_rules! impl_move_options_integer {
	($($ty:ty)*) => {
		$(
			impl $crate::options::sealed::Sealed for $ty {}

			impl $crate::options::CellChangeOptionsInteger for $ty {}
		)*
	};
}

impl_move_options_integer!(i8 u8);
