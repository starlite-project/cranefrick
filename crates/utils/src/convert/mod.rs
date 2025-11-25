#[cfg(feature = "convert")]
use core::convert::TryInto;

#[cfg(feature = "get_or_zero")]
mod get_or_zero;
#[cfg(feature = "into_range")]
mod into_range;

#[cfg(feature = "get_or_zero")]
pub use self::get_or_zero::*;
#[cfg(feature = "into_range")]
pub use self::into_range::*;

#[cfg(feature = "convert")]
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

#[cfg(feature = "convert")]
impl<T> Convert for T {}

#[cfg(feature = "convert")]
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

#[cfg(feature = "convert")]
impl<T> TryConvert for T {}
