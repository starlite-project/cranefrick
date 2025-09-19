use core::{convert::TryInto, marker::PhantomData, mem::MaybeUninit, pin::Pin};

use super::{New, TryNew};

pub unsafe fn by_raw<T>(f: impl FnOnce(Pin<&mut MaybeUninit<T>>)) -> impl New<Output = T> {
	struct FnNew<F, T> {
		f: F,
		marker: PhantomData<fn(T)>,
	}

	unsafe impl<F, T> New for FnNew<F, T>
	where
		F: FnOnce(Pin<&mut MaybeUninit<T>>),
	{
		type Output = T;

		unsafe fn new(self, this: Pin<&mut MaybeUninit<Self::Output>>) {
			(self.f)(this);
		}
	}

	FnNew {
		f,
		marker: PhantomData,
	}
}

pub fn by<T>(f: impl FnOnce() -> T) -> impl New<Output = T> {
	unsafe { by_raw(|mut this| this.set(MaybeUninit::new(f()))) }
}

pub fn from<T, U>(value: U) -> impl New<Output = T>
where
	T: From<U>,
{
	by(|| value.into())
}

pub fn of<T>(value: T) -> impl New<Output = T> {
	by(|| value)
}

pub fn default<T: Default>() -> impl New<Output = T> {
	by(T::default)
}

pub unsafe fn try_by_raw<T, E>(
	f: impl FnOnce(Pin<&mut MaybeUninit<T>>) -> Result<(), E>,
) -> impl TryNew<Output = T, Error = E> {
	struct FnTryNew<F, T, E> {
		f: F,
		marker: PhantomData<fn(T) -> E>,
	}

	unsafe impl<F, T, E> TryNew for FnTryNew<F, T, E>
	where
		F: FnOnce(Pin<&mut MaybeUninit<T>>) -> Result<(), E>,
	{
		type Error = E;
		type Output = T;

		unsafe fn try_new(
			self,
			this: Pin<&mut MaybeUninit<Self::Output>>,
		) -> Result<(), Self::Error> {
			(self.f)(this)
		}
	}

	FnTryNew {
		f,
		marker: PhantomData,
	}
}

pub fn try_by<T, E>(f: impl FnOnce() -> Result<T, E>) -> impl TryNew<Output = T, Error = E> {
	unsafe {
		try_by_raw(|this| {
			this.get_unchecked_mut().write(f()?);
			Ok(())
		})
	}
}

pub fn try_from<T, U>(value: U) -> impl TryNew<Output = T, Error = T::Error>
where
	T: TryFrom<U>,
{
	try_by(|| value.try_into())
}
