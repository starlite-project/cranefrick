#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

mod sealed;
mod serde_impl;

use core::{
	fmt::{Debug, Formatter, Result as FmtResult},
	marker::PhantomData,
};

use serde::{Deserialize, Serialize};

#[repr(transparent)]
pub struct Register<T>
where
	T: ?Sized + RegisterType,
{
	index: usize,
	marker: PhantomData<T>,
}

impl<T> Register<T>
where
	T: ?Sized + RegisterType,
{
	#[must_use]
	pub const fn new(index: usize) -> Self {
		Self {
			index,
			marker: PhantomData,
		}
	}

	#[must_use]
	pub const fn index(self) -> usize {
		self.index
	}

	#[must_use]
	pub const fn cast<U>(self) -> Register<U>
	where
		U: ?Sized + RegisterType,
	{
		Register::new(self.index())
	}
}

impl<T> Clone for Register<T>
where
	T: ?Sized + RegisterType,
{
	fn clone(&self) -> Self {
		*self
	}
}

impl<T> Copy for Register<T> where T: ?Sized + RegisterType {}

impl<T> Debug for Register<T>
where
	T: ?Sized + RegisterType,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_tuple("Register").field(&self.index).finish()
	}
}

impl<T> Eq for Register<T> where T: ?Sized + RegisterType {}

impl<T> From<usize> for Register<T>
where
	T: ?Sized + RegisterType,
{
	fn from(value: usize) -> Self {
		Self::new(value)
	}
}

impl<T> From<Register<T>> for usize
where
	T: ?Sized + RegisterType,
{
	fn from(value: Register<T>) -> Self {
		value.index()
	}
}

impl<T> PartialEq for Register<T>
where
	T: ?Sized + RegisterType,
{
	fn eq(&self, other: &Self) -> bool {
		PartialEq::eq(&self.index, &other.index)
	}
}

impl<T> PartialEq<usize> for Register<T>
where
	T: ?Sized + RegisterType,
{
	fn eq(&self, other: &usize) -> bool {
		PartialEq::eq(&self.index, other)
	}
}

impl<T> PartialEq<Register<T>> for usize
where
	T: ?Sized + RegisterType,
{
	fn eq(&self, other: &Register<T>) -> bool {
		PartialEq::eq(self, &other.index)
	}
}

pub enum Bool {}

impl RegisterType for Bool {}

pub enum Int {}

impl RegisterType for Int {}

pub enum Pointer {}

impl RegisterType for Pointer {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BinaryOperation {
	Add,
	Sub,
	Mul,
	BitwiseAnd,
}

pub trait RegisterType: self::sealed::Sealed {}
