use core::{
	fmt::{Debug, Formatter, Result as FmtResult, Write as _},
	mem,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PackedOption<T: ReservedValue>(T);

impl<T: ReservedValue> PackedOption<T> {
	pub fn is_none(&self) -> bool {
		self.0.is_reserved_value()
	}

	pub fn is_some(&self) -> bool {
		!self.is_none()
	}

	pub fn expand(self) -> Option<T> {
		self.is_some().then_some(self.0)
	}

	pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Option<U> {
		self.expand().map(f)
	}

	#[track_caller]
	pub fn unwrap(self) -> T {
		self.expand().unwrap()
	}

	#[track_caller]
	pub fn expect(self, msg: &str) -> T {
		self.expand().expect(msg)
	}

	#[must_use]
	pub fn take(&mut self) -> Self {
		mem::take(self)
	}
}

impl<T> Debug for PackedOption<T>
where
	T: Debug + ReservedValue,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		if self.is_none() {
			f.write_str("None")
		} else {
			f.write_str("Some(")?;
			Debug::fmt(&self.0, f)?;
			f.write_char(')')
		}
	}
}

impl<T: ReservedValue> Default for PackedOption<T> {
	fn default() -> Self {
		Self(T::reserved_value())
	}
}

impl<T: ReservedValue> From<T> for PackedOption<T> {
	fn from(value: T) -> Self {
		debug_assert!(
			!value.is_reserved_value(),
			"can't make a PackedOption from the reserved value"
		);
		Self(value)
	}
}

impl<T: ReservedValue> From<Option<T>> for PackedOption<T> {
	fn from(value: Option<T>) -> Self {
		value.map_or_else(Self::default, Self::from)
	}
}

impl<T: ReservedValue> From<PackedOption<T>> for Option<T> {
	fn from(value: PackedOption<T>) -> Self {
		value.expand()
	}
}

pub trait ReservedValue {
	fn reserved_value() -> Self;

	fn is_reserved_value(&self) -> bool;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Debug, PartialEq, Eq)]
	#[repr(transparent)]
	struct NoCopy(u32);

	impl ReservedValue for NoCopy {
		fn reserved_value() -> Self {
			Self(13)
		}

		fn is_reserved_value(&self) -> bool {
			matches!(self.0, 13)
		}
	}

	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	#[repr(transparent)]
	struct Entity(u32);

	impl ReservedValue for Entity {
		fn reserved_value() -> Self {
			Self(13)
		}

		fn is_reserved_value(&self) -> bool {
			matches!(self.0, 13)
		}
	}

	#[test]
	fn moves() {
		let x = NoCopy(3);
		let some: PackedOption<_> = x.into();
		assert!(!some.is_none());
		assert_eq!(some.expand(), Some(NoCopy(3)));

		let none: PackedOption<NoCopy> = None.into();
		assert!(none.is_none());
		assert_eq!(none.expand(), None);
	}

	#[test]
	fn copies() {
		let x = Entity(2);
		let some: PackedOption<_> = x.into();

		assert_eq!(some.expand(), x.into());
		assert_eq!(some, x.into());
	}
}
