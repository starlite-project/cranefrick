#![allow(clippy::new_ret_no_self, clippy::wrong_self_convention)]

mod copy_new;
mod factories;
mod move_new;

use alloc::{boxed::Box, rc::Rc, sync::Arc};
use core::{convert::Infallible, mem::MaybeUninit, ops::Deref, pin::Pin};

pub use self::{copy_new::*, factories::*, move_new::*};

#[must_use = "`New`s do nothing until emplaced into storage"]
pub unsafe trait New: Sized {
	type Output;

	unsafe fn new(self, this: Pin<&mut MaybeUninit<Self::Output>>);

	fn with<F>(self, post: F) -> With<Self, F>
	where
		F: FnOnce(Pin<&mut Self::Output>),
	{
		With(self, post)
	}
}

#[must_use = "`New`s do nothing until emplaced into storage"]
pub unsafe trait TryNew: Sized {
	type Output;
	type Error;

	unsafe fn try_new(self, this: Pin<&mut MaybeUninit<Self::Output>>) -> Result<(), Self::Error>;

	fn try_with<F>(self, post: F) -> TryWith<Self, F>
	where
		F: FnOnce(Pin<&mut Self::Output>) -> Result<(), Self::Error>,
	{
		TryWith(self, post)
	}
}

unsafe impl<N: New> TryNew for N {
	type Error = Infallible;
	type Output = N::Output;

	unsafe fn try_new(self, this: Pin<&mut MaybeUninit<Self::Output>>) -> Result<(), Self::Error> {
		unsafe { self.new(this) };
		Ok(())
	}
}

pub trait Emplace<T>: Deref + Sized {
	type Output: Deref<Target = Self::Target>;

	fn emplace<N>(n: N) -> Self::Output
	where
		N: New<Output = T>,
	{
		match Self::try_emplace(n) {
			Ok(x) => x,
			Err(e) => match e {},
		}
	}

	fn try_emplace<N>(n: N) -> Result<Self::Output, N::Error>
	where
		N: TryNew<Output = T>;
}

impl<T> Emplace<T> for Box<T> {
	type Output = Pin<Self>;

	fn try_emplace<N>(n: N) -> Result<Self::Output, N::Error>
	where
		N: TryNew<Output = T>,
	{
		let mut uninit = Box::new(MaybeUninit::<T>::uninit());

		unsafe {
			let pinned = Pin::new_unchecked(&mut *uninit);
			n.try_new(pinned)?;
			Ok(Pin::new_unchecked(Self::from_raw(
				Box::into_raw(uninit).cast::<T>(),
			)))
		}
	}
}

impl<T> Emplace<T> for Rc<T> {
	type Output = Pin<Self>;

	fn try_emplace<N>(n: N) -> Result<Self::Output, N::Error>
	where
		N: TryNew<Output = T>,
	{
		let uninit = Rc::new(MaybeUninit::<T>::uninit());

		unsafe {
			let pinned = Pin::new_unchecked(&mut *Rc::as_ptr(&uninit).cast_mut());
			n.try_new(pinned)?;
			Ok(Pin::new_unchecked(Self::from_raw(
				Rc::into_raw(uninit).cast::<T>(),
			)))
		}
	}
}

impl<T> Emplace<T> for Arc<T> {
	type Output = Pin<Self>;

	fn try_emplace<N>(n: N) -> Result<Self::Output, N::Error>
	where
		N: TryNew<Output = T>,
	{
		let uninit = Arc::new(MaybeUninit::<T>::uninit());

		unsafe {
			let pinned = Pin::new_unchecked(&mut *Arc::as_ptr(&uninit).cast_mut());
			n.try_new(pinned)?;
			Ok(Pin::new_unchecked(Self::from_raw(
				Arc::into_raw(uninit).cast::<T>(),
			)))
		}
	}
}

pub struct With<N, F>(N, F);

unsafe impl<N: New, F> New for With<N, F>
where
	F: FnOnce(Pin<&mut N::Output>),
{
	type Output = N::Output;

	unsafe fn new(self, mut this: Pin<&mut MaybeUninit<Self::Output>>) {
		unsafe { self.0.new(this.as_mut()) };

		let this = unsafe { this.map_unchecked_mut(|x| x.assume_init_mut()) };
		(self.1)(this);
	}
}

pub struct TryWith<N, F>(N, F);

unsafe impl<N: TryNew, F> TryNew for TryWith<N, F>
where
	F: FnOnce(Pin<&mut N::Output>) -> Result<(), N::Error>,
{
	type Error = N::Error;
	type Output = N::Output;

	unsafe fn try_new(
		self,
		mut this: Pin<&mut MaybeUninit<Self::Output>>,
	) -> Result<(), Self::Error> {
		unsafe { self.0.try_new(this.as_mut()) }?;

		let this = unsafe { this.map_unchecked_mut(|x| x.assume_init_mut()) };

		(self.1)(this)
	}
}

pub trait Swap<Rhs = Self> {
	fn swap_with(self: Pin<&mut Self>, src: Pin<&mut Rhs>);
}
