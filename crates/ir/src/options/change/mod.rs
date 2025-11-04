mod sealed;
mod serde;

use core::{
	fmt::{Debug, Display, Formatter, Result as FmtResult, Write as _},
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

impl<T: OffsetCellPrimitive> FactoredOffsetCellOptions<T> {
	pub const fn factor(self) -> T {
		self.value
	}

	pub const fn into_value(self) -> ValuedOffsetCellOptions<T> {
		OffsetCellOptions::new(self.value, self.offset)
	}
}

impl<T: OffsetCellPrimitive> ValuedOffsetCellOptions<T> {
	pub const fn value(self) -> T {
		self.value
	}

	pub const fn into_factor(self) -> FactoredOffsetCellOptions<T> {
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
	T: Display + OffsetCellPrimitive,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		Display::fmt(self, f)
	}
}

impl<T, Marker: OffsetCellMarker> Display for OffsetCellOptions<T, Marker>
where
	T: Display + OffsetCellPrimitive,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let (value, offset) = self.into_parts();

		f.write_char('(')?;
		f.write_char(Marker::sign())?;
		Display::fmt(&value, f)?;
		f.write_str(", ")?;
		Display::fmt(&offset, f)?;
		f.write_char(')')
	}
}

impl<T: OffsetCellPrimitive, Marker: OffsetCellMarker> Eq for OffsetCellOptions<T, Marker> {}

impl<T: OffsetCellPrimitive, Marker: OffsetCellMarker> PartialEq for OffsetCellOptions<T, Marker> {
	fn eq(&self, other: &Self) -> bool {
		PartialEq::eq(&self.value, &other.value) && PartialEq::eq(&self.offset, &other.offset)
	}
}

pub enum Factor {}

impl OffsetCellMarker for Factor {
	fn sign() -> char {
		'*'
	}
}

pub enum Value {}

impl OffsetCellMarker for Value {
	fn sign() -> char {
		'+'
	}
}

pub trait OffsetCellMarker: self::sealed::MarkerSealed {
	fn sign() -> char;
}

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
