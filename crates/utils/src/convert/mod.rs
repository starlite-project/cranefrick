use core::convert::TryInto;

#[cfg(feature = "get_or_zero")]
mod get_or_zero;
#[cfg(feature = "unwrap_from")]
mod unwrap_from;

#[cfg(feature = "get_or_zero")]
pub use self::get_or_zero::*;
#[cfg(feature = "unwrap_from")]
pub use self::unwrap_from::*;

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
