use core::fmt::Debug;

pub trait UnwrapFrom<T>: Sized {
	fn unwrap_from(value: T) -> Self;
}

impl<T, U> UnwrapFrom<T> for U
where
	U: TryFrom<T>,
	<U as TryFrom<T>>::Error: Debug,
{
	fn unwrap_from(value: T) -> Self {
		U::try_from(value).unwrap()
	}
}

pub trait UnwrapInto<T> {
	fn unwrap_into(self) -> T;
}

impl<T, U> UnwrapInto<U> for T
where
	U: UnwrapFrom<T>,
{
	fn unwrap_into(self) -> U {
		U::unwrap_from(self)
	}
}
