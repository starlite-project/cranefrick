use core::convert::TryInto;

#[cfg(feature = "nightly")]
mod get_or_zero;
#[cfg(feature = "nightly")]
mod into_range;

#[cfg(feature = "nightly")]
pub use self::{get_or_zero::*, into_range::*};

pub trait Convert
where
	Self: Sized,
{
	#[inline]
	fn convert<T>(self) -> T
	where
		Self: Into<T>,
		T: Sized,
	{
		Into::<T>::into(self)
	}
}

impl<T> Convert for T {}

pub trait TryConvert
where
	Self: Sized,
{
	#[inline]
	fn try_convert<T>(self) -> Result<T, Self::Error>
	where
		Self: TryInto<T>,
		T: Sized,
	{
		TryInto::<T>::try_into(self)
	}
}

impl<T> TryConvert for T {}
