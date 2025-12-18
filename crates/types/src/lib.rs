#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

mod sealed;
mod serde_impl;

use core::{
	fmt::{Debug, Formatter, Result as FmtResult},
	marker::PhantomData,
};

use frick_spec::{POINTER_SIZE, TAPE_SIZE};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Immediate {
	value: u64,
	size: u32,
}

impl Immediate {
	pub const CELL_ZERO: Self = Self::cell(0);
	pub const TAPE_SIZE_MINUS_ONE: Self = Self::pointer(TAPE_SIZE as u64 - 1);

	#[must_use]
	pub const fn new(value: u64, size: u32) -> Self {
		Self { value, size }
	}

	#[must_use]
	pub const fn pointer(value: u64) -> Self {
		Self::new(value, POINTER_SIZE as u32)
	}

	#[must_use]
	pub const fn cell(value: u64) -> Self {
		Self::new(value, 8)
	}

	#[must_use]
	pub const fn value(self) -> u64 {
		self.value
	}

	#[must_use]
	pub const fn size(self) -> u32 {
		self.size
	}
}

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

pub enum Any {}

impl RegisterType for Any {
	type RustType = core::convert::Infallible;
}

pub enum Bool {}

impl RegisterType for Bool {
	type RustType = bool;
}

pub enum Int {}

impl RegisterType for Int {
	type RustType = Immediate;
}

pub enum Pointer {}

impl RegisterType for Pointer {
	type RustType = *const ();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BinaryOperation {
	Add,
	Sub,
	Mul,
	BitwiseAnd,
	BitwiseShl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegisterTypeEnum {
	Any,
	Bool,
	Int(Option<usize>),
	Pointer,
}

pub enum RegOrImm<T: RegisterType> {
	Reg(Register<T>),
	Imm(T::RustType),
}

impl<T: RegisterType> RegOrImm<T> {
	#[must_use]
	pub const fn reg(r: Register<T>) -> Self {
		Self::Reg(r)
	}

	#[must_use]
	pub const fn imm(value: T::RustType) -> Self {
		Self::Imm(value)
	}
}

impl<T: RegisterType> Clone for RegOrImm<T>
where
	T::RustType: Clone,
{
	fn clone(&self) -> Self {
		match self {
			Self::Reg(r) => Self::Reg(*r),
			Self::Imm(i) => Self::Imm(i.clone()),
		}
	}
}

impl<T: RegisterType> Copy for RegOrImm<T> where T::RustType: Copy {}

impl<T: RegisterType> Debug for RegOrImm<T>
where
	T::RustType: Debug,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Reg(r) => f.debug_tuple("Reg").field(&r).finish(),
			Self::Imm(i) => f.debug_tuple("Imm").field(&i).finish(),
		}
	}
}

impl<T: RegisterType> Eq for RegOrImm<T> where T::RustType: Eq {}

impl<T: RegisterType> PartialEq for RegOrImm<T>
where
	T::RustType: PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Reg(lhs), Self::Reg(rhs)) => lhs == rhs,
			(Self::Imm(lhs), Self::Imm(rhs)) => lhs == rhs,
			_ => false,
		}
	}
}

pub trait RegisterType: self::sealed::Sealed {
	type RustType;
}
