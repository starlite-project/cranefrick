use core::num::{NonZero, ZeroablePrimitive};

pub trait GetOrZero<T> {
	fn get_or_zero(self) -> T;
}

impl<T> GetOrZero<T> for Option<NonZero<T>>
where
	T: Default + ZeroablePrimitive,
{
	fn get_or_zero(self) -> T {
		match self {
			None => T::default(),
			Some(x) => x.get(),
		}
	}
}

impl<T> GetOrZero<T> for Option<T>
where
	T: Default + ZeroablePrimitive,
{
	fn get_or_zero(self) -> T {
		self.unwrap_or_default()
	}
}

impl<T: ZeroablePrimitive> GetOrZero<T> for NonZero<T> {
	fn get_or_zero(self) -> T {
		self.get()
	}
}

impl<T: ZeroablePrimitive> GetOrZero<T> for T {
	fn get_or_zero(self) -> T {
		self
	}
}
