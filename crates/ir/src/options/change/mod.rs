mod sealed;

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeCellOptions<T: ChangeCellOptionsPrimitive = u8> {
	value: T,
	offset: i32,
}

impl<T: ChangeCellOptionsPrimitive> ChangeCellOptions<T> {
	pub const fn new(value: T, offset: i32) -> Self {
		Self { value, offset }
	}

	pub const fn value(self) -> T {
		self.value
	}

	pub const fn offset(self) -> i32 {
		self.offset
	}

	pub const fn into_parts(self) -> (T, i32) {
		(self.value, self.offset)
	}

	pub fn is_default(self) -> bool {
		self.value() == T::default() && matches!(self.offset(), 0)
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
		PartialEq::eq(&self.value, &other.value) && PartialEq::eq(&self.offset, &other.offset)
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
