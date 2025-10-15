mod sealed;
mod serde;

use std::{
	fmt::{Debug, Formatter, Result as FmtResult},
	marker::PhantomData,
	ops::RangeInclusive,
};

pub struct ChangeCellOptions<T: ChangeCellPrimitive, Marker: ChangeCellMarker> {
	value: T,
	offset: i32,
	marker: PhantomData<Marker>,
}

impl<T: ChangeCellPrimitive, Marker: ChangeCellMarker> ChangeCellOptions<T, Marker> {
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

	#[must_use]
	pub const fn is_offset(self) -> bool {
		!matches!(self.offset(), 0)
	}

	pub fn is_default(self) -> bool {
		self.value == T::default() && !self.is_offset()
	}
}

impl<T: ChangeCellPrimitive> ChangeCellOptions<T, Factor> {
	pub const fn factor(self) -> T {
		self.value
	}

	pub const fn into_value(self) -> ChangeCellOptions<T, Value> {
		ChangeCellOptions::new(self.value, self.offset)
	}
}

impl<T: ChangeCellPrimitive> ChangeCellOptions<T, Value> {
	pub const fn value(self) -> T {
		self.value
	}

	pub const fn into_factor(self) -> ChangeCellOptions<T, Factor> {
		ChangeCellOptions::new(self.value, self.offset)
	}
}

impl<T: ChangeCellPrimitive, Marker: ChangeCellMarker> Clone for ChangeCellOptions<T, Marker> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: ChangeCellPrimitive, Marker: ChangeCellMarker> Copy for ChangeCellOptions<T, Marker> {}

impl<T, Marker: ChangeCellMarker> Debug for ChangeCellOptions<T, Marker>
where
	T: ChangeCellPrimitive + Debug,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_struct("ChangeCellOptions")
			.field("value", &self.value)
			.field("offset", &self.offset)
			.finish()
	}
}

impl<T: ChangeCellPrimitive, Marker: ChangeCellMarker> Eq for ChangeCellOptions<T, Marker> {}

impl<T: ChangeCellPrimitive, Marker: ChangeCellMarker> PartialEq for ChangeCellOptions<T, Marker> {
	fn eq(&self, other: &Self) -> bool {
		PartialEq::eq(&self.value, &other.value) && PartialEq::eq(&self.offset, &other.offset)
	}
}

pub enum Factor {}

pub enum Value {}

pub trait ChangeCellMarker: self::sealed::MarkerSealed {}

impl<T: self::sealed::MarkerSealed> ChangeCellMarker for T {}

pub trait ChangeCellPrimitive: Copy + Default + Eq + self::sealed::PrimitiveSealed {}

impl<T> ChangeCellPrimitive for T where T: Copy + Default + Eq + self::sealed::PrimitiveSealed {}

pub type FactoredChangeCellOptions<T> = ChangeCellOptions<T, Factor>;

pub type ValuedChangeCellOptions<T> = ChangeCellOptions<T, Value>;

pub fn is_range<T: ChangeCellPrimitive, Marker: ChangeCellMarker>(
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

pub fn get_range<T: ChangeCellPrimitive, Marker: ChangeCellMarker>(
	values: &[ChangeCellOptions<T, Marker>],
) -> Option<RangeInclusive<i32>> {
	assert!(values.len() > 1);

	let first = values.first().copied()?;

	let last = values.last().copied()?;

	Some(first.offset()..=last.offset())
}
