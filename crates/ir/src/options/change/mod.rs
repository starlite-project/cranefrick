mod sealed;
mod serde;

use std::{
	fmt::{Debug, Formatter, Result as FmtResult},
	marker::PhantomData,
};

pub struct OffsetCellOptions<T: OffsetCellPrimitive, Marker: OffsetCellMarker> {
	value: T,
	offset: i32,
	marker: PhantomData<Marker>,
}

impl<T: OffsetCellPrimitive, Marker: OffsetCellMarker> OffsetCellOptions<T, Marker> {
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

	pub const fn offset_mut(&mut self) -> &mut i32 {
		&mut self.offset
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

	#[must_use]
	pub fn wrapping_add(self, rhs: Self) -> Self {
		Self::new(T::wrapping_add(self.value, rhs.value), self.offset)
	}
}

impl<T: OffsetCellPrimitive> OffsetCellOptions<T, Factor> {
	pub const fn factor(self) -> T {
		self.value
	}

	pub const fn into_value(self) -> OffsetCellOptions<T, Value> {
		OffsetCellOptions::new(self.value, self.offset)
	}
}

impl<T: OffsetCellPrimitive> OffsetCellOptions<T, Value> {
	pub const fn value(self) -> T {
		self.value
	}

	pub const fn into_factor(self) -> OffsetCellOptions<T, Factor> {
		OffsetCellOptions::new(self.value, self.offset)
	}
}

impl<T: OffsetCellPrimitive, Marker: OffsetCellMarker> Clone for OffsetCellOptions<T, Marker> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: OffsetCellPrimitive, Marker: OffsetCellMarker> Copy for OffsetCellOptions<T, Marker> {}

impl<T, Marker: OffsetCellMarker> Debug for OffsetCellOptions<T, Marker>
where
	T: OffsetCellPrimitive + Debug,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_struct("ChangeCellOptions")
			.field("value", &self.value)
			.field("offset", &self.offset)
			.finish()
	}
}

impl<T: OffsetCellPrimitive, Marker: OffsetCellMarker> Eq for OffsetCellOptions<T, Marker> {}

impl<T: OffsetCellPrimitive, Marker: OffsetCellMarker> PartialEq for OffsetCellOptions<T, Marker> {
	fn eq(&self, other: &Self) -> bool {
		PartialEq::eq(&self.value, &other.value) && PartialEq::eq(&self.offset, &other.offset)
	}
}

pub enum Factor {}

pub enum Value {}

pub trait OffsetCellMarker: self::sealed::MarkerSealed {}

impl<T: self::sealed::MarkerSealed> OffsetCellMarker for T {}

pub trait OffsetCellPrimitive: Copy + Default + Eq + self::sealed::PrimitiveSealed {
	#[must_use]
	fn wrapping_add(self, rhs: Self) -> Self;
}

pub type FactoredOffsetCellOptions<T> = OffsetCellOptions<T, Factor>;

pub type ValuedOffsetCellOptions<T> = OffsetCellOptions<T, Value>;

macro_rules! impl_change_cell_primitive {
	($($ty:ty)*) => {
		$(
			impl $crate::options::change::OffsetCellPrimitive for $ty {
				fn wrapping_add(self, rhs: Self) -> Self {
					<$ty>::wrapping_add(self, rhs)
				}
			}
		)*
	};
}

impl_change_cell_primitive!(i8 u8);
