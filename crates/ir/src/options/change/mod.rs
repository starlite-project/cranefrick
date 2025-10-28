mod sealed;
mod serde;

use std::{
	fmt::{Debug, Formatter, Result as FmtResult},
	marker::PhantomData,
};

pub struct OffsetCellOptions<T: ChangeCellPrimitive, Marker: ChangeCellMarker> {
	value: T,
	offset: i32,
	marker: PhantomData<Marker>,
}

impl<T: ChangeCellPrimitive, Marker: ChangeCellMarker> OffsetCellOptions<T, Marker> {
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

impl<T: ChangeCellPrimitive> OffsetCellOptions<T, Factor> {
	pub const fn factor(self) -> T {
		self.value
	}

	pub const fn into_value(self) -> OffsetCellOptions<T, Value> {
		OffsetCellOptions::new(self.value, self.offset)
	}
}

impl<T: ChangeCellPrimitive> OffsetCellOptions<T, Value> {
	pub const fn value(self) -> T {
		self.value
	}

	pub const fn into_factor(self) -> OffsetCellOptions<T, Factor> {
		OffsetCellOptions::new(self.value, self.offset)
	}
}

impl<T: ChangeCellPrimitive, Marker: ChangeCellMarker> Clone for OffsetCellOptions<T, Marker> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: ChangeCellPrimitive, Marker: ChangeCellMarker> Copy for OffsetCellOptions<T, Marker> {}

impl<T, Marker: ChangeCellMarker> Debug for OffsetCellOptions<T, Marker>
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

impl<T: ChangeCellPrimitive, Marker: ChangeCellMarker> Eq for OffsetCellOptions<T, Marker> {}

impl<T: ChangeCellPrimitive, Marker: ChangeCellMarker> PartialEq for OffsetCellOptions<T, Marker> {
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

pub type FactoredOffsetCellOptions<T> = OffsetCellOptions<T, Factor>;

pub type ValuedOffsetCellOptions<T> = OffsetCellOptions<T, Value>;
