mod sealed;

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeCellOptions<T: ChangeCellOptionsPrimitive = u8> {
	offset_value_by: ChangeCellValue<T>,
	offset: i32,
}

impl<T: ChangeCellOptionsPrimitive> ChangeCellOptions<T> {
	pub const fn new(value_offset: ChangeCellValue<T>, offset: i32) -> Self {
		Self {
			offset_value_by: value_offset,
			offset,
		}
	}

	pub const fn new_factor(value: T, offset: i32) -> Self {
		Self::new(ChangeCellValue::Factor(value), offset)
	}

	pub const fn new_value(value: T, offset: i32) -> Self {
		Self::new(ChangeCellValue::Value(value), offset)
	}

	pub const fn value(self) -> Option<T> {
		self.offset_value_by.value()
	}

	pub const fn factor(self) -> Option<T> {
		self.offset_value_by.factor()
	}

	pub const fn offset_value_by(self) -> ChangeCellValue<T> {
		self.offset_value_by
	}

	pub const fn offset(self) -> i32 {
		self.offset
	}

	pub const fn into_parts(self) -> (T, i32) {
		(self.offset_value_by.inner_value(), self.offset)
	}

	pub const fn inner_value(self) -> T {
		self.offset_value_by().inner_value()
	}

	pub fn is_default(self) -> bool {
		self.offset_value_by.inner_value() == T::default() && matches!(self.offset(), 0)
	}

	#[must_use]
	pub const fn into_factored(self) -> Self {
		Self::new(ChangeCellValue::Factor(self.inner_value()), self.offset())
	}

	#[must_use]
	pub const fn into_value(self) -> Self {
		Self::new(ChangeCellValue::Value(self.inner_value()), self.offset())
	}
}

impl<T: ChangeCellOptionsPrimitive> Clone for ChangeCellOptions<T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: ChangeCellOptionsPrimitive> Copy for ChangeCellOptions<T> {}

impl<T: ChangeCellOptionsPrimitive> Eq for ChangeCellOptions<T> {}

impl<T: ChangeCellOptionsPrimitive> PartialEq for ChangeCellOptions<T> {
	fn eq(&self, other: &Self) -> bool {
		PartialEq::eq(&self.offset_value_by, &other.offset_value_by)
			&& PartialEq::eq(&self.offset, &other.offset)
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ChangeCellValue<T: ChangeCellOptionsPrimitive = u8> {
	Value(T),
	Factor(T),
}

impl<T: ChangeCellOptionsPrimitive> ChangeCellValue<T> {
	pub const fn value(self) -> Option<T> {
		match self {
			Self::Value(value) => Some(value),
			Self::Factor(..) => None,
		}
	}

	pub const fn factor(self) -> Option<T> {
		match self {
			Self::Factor(factor) => Some(factor),
			Self::Value(..) => None,
		}
	}

	pub const fn inner_value(self) -> T {
		match self {
			Self::Factor(inner) | Self::Value(inner) => inner,
		}
	}
}

impl<T: ChangeCellOptionsPrimitive> Clone for ChangeCellValue<T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: ChangeCellOptionsPrimitive> Copy for ChangeCellValue<T> {}

impl<T: ChangeCellOptionsPrimitive> Eq for ChangeCellValue<T> {}

impl<T: ChangeCellOptionsPrimitive> PartialEq for ChangeCellValue<T> {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Factor(lhs), Self::Factor(rhs)) | (Self::Value(lhs), Self::Value(rhs)) => {
				PartialEq::eq(&lhs, &rhs)
			}
			_ => false,
		}
	}
}

pub trait ChangeCellOptionsPrimitive: Copy + Default + Eq + self::sealed::Sealed {}

impl<T> ChangeCellOptionsPrimitive for T where T: Copy + Default + Eq + self::sealed::Sealed {}

pub fn is_range<T: ChangeCellOptionsPrimitive>(values: &[ChangeCellOptions<T>]) -> bool {
	let Some(range) = get_range(values) else {
		return false;
	};

	for offset in values.iter().map(|options| options.offset()) {
		if !range.contains(&offset) {
			return false;
		}
	}

	let len_of_values = values.len();

	range.count() == len_of_values
}

pub fn get_range<T: ChangeCellOptionsPrimitive>(
	values: &[ChangeCellOptions<T>],
) -> Option<RangeInclusive<i32>> {
	assert!(values.len() > 1);

	let first = values.first().copied()?;

	let last = values.last().copied()?;

	Some(first.offset()..=last.offset())
}
