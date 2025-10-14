mod sealed;

use std::{
	fmt::{Debug, Formatter, Result as FmtResult},
	marker::PhantomData,
	ops::RangeInclusive,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ChangeCellOptions<T: ChangeCellOptionsPrimitive, Marker: ChangeCellMarker> {
	value: T,
	offset: i32,
	marker: PhantomData<Marker>,
}

impl<T: ChangeCellOptionsPrimitive, Marker: ChangeCellMarker> ChangeCellOptions<T, Marker> {
	pub const fn new(value: T, offset: i32) -> Self {
		Self {
			value,
			offset,
			marker: PhantomData,
		}
	}

	pub const fn offset(self) -> i32 {
		self.offset
	}

	pub const fn into_parts(self) -> (T, i32) {
		(self.value, self.offset)
	}

	pub fn is_default(self) -> bool {
		self.value == T::default() && matches!(self.offset(), 0)
	}
}

impl<T: ChangeCellOptionsPrimitive> ChangeCellOptions<T, Factor> {
	pub const fn factor(self) -> T {
		self.value
	}

	pub const fn into_value(self) -> ChangeCellOptions<T, Value> {
		ChangeCellOptions::new(self.value, self.offset)
	}
}

impl<T: ChangeCellOptionsPrimitive> ChangeCellOptions<T, Value> {
	pub const fn value(self) -> T {
		self.value
	}

	pub const fn into_factor(self) -> ChangeCellOptions<T, Factor> {
		ChangeCellOptions::new(self.value, self.offset)
	}
}

impl<T: ChangeCellOptionsPrimitive, Marker: ChangeCellMarker> Clone
	for ChangeCellOptions<T, Marker>
{
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: ChangeCellOptionsPrimitive, Marker: ChangeCellMarker> Copy
	for ChangeCellOptions<T, Marker>
{
}

impl<T, Marker: ChangeCellMarker> Debug for ChangeCellOptions<T, Marker>
where
	T: ChangeCellOptionsPrimitive + Debug,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_struct("ChangeCellOptions")
			.field("value", &self.value)
			.field("offset", &self.offset)
			.finish()
	}
}

impl<T: ChangeCellOptionsPrimitive, Marker: ChangeCellMarker> Eq for ChangeCellOptions<T, Marker> {}

impl<T: ChangeCellOptionsPrimitive, Marker: ChangeCellMarker> PartialEq
	for ChangeCellOptions<T, Marker>
{
	fn eq(&self, other: &Self) -> bool {
		PartialEq::eq(&self.value, &other.value) && PartialEq::eq(&self.offset, &other.offset)
	}
}

pub enum Factor {}

pub enum Value {}

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

pub trait ChangeCellMarker: self::sealed::MarkerSealed {}

impl<T: self::sealed::MarkerSealed> ChangeCellMarker for T {}

pub trait ChangeCellOptionsPrimitive: Copy + Default + Eq + self::sealed::PrimitiveSealed {}

impl<T> ChangeCellOptionsPrimitive for T where T: Copy + Default + Eq + self::sealed::PrimitiveSealed
{}

pub fn is_range<T: ChangeCellOptionsPrimitive, Marker: ChangeCellMarker>(
	values: &[ChangeCellOptions<T, Marker>],
) -> bool {
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

pub fn get_range<T: ChangeCellOptionsPrimitive, Marker: ChangeCellMarker>(
	values: &[ChangeCellOptions<T, Marker>],
) -> Option<RangeInclusive<i32>> {
	assert!(values.len() > 1);

	let first = values.first().copied()?;

	let last = values.last().copied()?;

	Some(first.offset()..=last.offset())
}
