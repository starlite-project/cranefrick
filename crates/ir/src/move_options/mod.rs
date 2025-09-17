mod sealed;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MoveOptions<T: MoveOptionsInteger = u8> {
	#[serde(skip_serializing_if = "is_one")]
	factor: T,
	offset: i32,
}

impl<T: MoveOptionsInteger> MoveOptions<T> {
	pub(crate) const fn new(factor: T, offset: i32) -> Self {
		Self { factor, offset }
	}

	#[must_use]
	pub const fn factor(self) -> T {
		self.factor
	}

	#[must_use]
	pub const fn offset(&self) -> i32 {
		self.offset
	}

	#[must_use]
	pub const fn into_parts(self) -> (T, i32) {
		(self.factor, self.offset)
	}

	pub fn is_default(self) -> bool {
		self.factor() == T::default() && matches!(self.offset(), 0)
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
		PartialEq::eq(&self.factor, &other.factor) && PartialEq::eq(&self.offset, &other.offset)
	}
}

pub trait MoveOptionsInteger: Copy + Default + Eq + self::sealed::Sealed {
	const ONE: Self;
}

fn is_one<T: MoveOptionsInteger>(value: &T) -> bool {
	value == &T::ONE
}

macro_rules! impl_move_options_integer {
	($($ty:ty)*) => {
		$(
			impl $crate::move_options::sealed::Sealed for $ty {}

			impl $crate::move_options::MoveOptionsInteger for $ty {
				const ONE: Self = 1;
			}
		)*
	};
}

impl_move_options_integer!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128);
