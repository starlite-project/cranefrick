use core::{
	fmt::{Debug, Display, Formatter, Result as FmtResult, Write as _},
	ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign},
};

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct PhysicalRegister {
	bits: u8,
}

impl PhysicalRegister {
	pub const MAX: usize = (1 << Self::MAX_BITS) - 1;
	pub const MAX_BITS: usize = 6;
	pub const NUM_INDEX: usize = 1 << (Self::MAX_BITS + 2);

	#[must_use]
	pub const fn new(hw_enc: usize, class: RegisterClass) -> Self {
		debug_assert!(hw_enc <= Self::MAX);
		Self {
			bits: ((class as u8) << Self::MAX_BITS) | (hw_enc as u8),
		}
	}

	#[must_use]
	pub const fn hardware_encode(self) -> usize {
		self.bits as usize & Self::MAX
	}

	#[must_use]
	pub const fn class(self) -> RegisterClass {
		match (self.bits >> Self::MAX_BITS) & 0b11 {
			0 => RegisterClass::Int,
			1 => RegisterClass::Float,
			2 => RegisterClass::Vector,
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub const fn index(self) -> usize {
		self.bits as usize
	}

	#[must_use]
	pub const fn from_index(index: usize) -> Self {
		Self {
			bits: (index & (Self::NUM_INDEX - 1)) as u8,
		}
	}

	#[must_use]
	pub const fn invalid() -> Self {
		Self::new(Self::MAX, RegisterClass::Int)
	}
}

impl Debug for PhysicalRegister {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str("PhysicalRegister(hw = ")?;
		Display::fmt(&self.hardware_encode(), f)?;
		f.write_str(", class = ")?;
		Debug::fmt(&self.class(), f)?;
		f.write_str(", index = ")?;
		Display::fmt(&self.index(), f)?;
		f.write_char(')')
	}
}

impl Default for PhysicalRegister {
	fn default() -> Self {
		Self::invalid()
	}
}

impl Display for PhysicalRegister {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let class = match self.class() {
			RegisterClass::Int => 'i',
			RegisterClass::Float => 'f',
			RegisterClass::Vector => 'v',
		};

		f.write_char('p')?;
		Display::fmt(&self.hardware_encode(), f)?;
		f.write_char(class)
	}
}

impl From<usize> for PhysicalRegister {
	fn from(value: usize) -> Self {
		Self::from_index(value)
	}
}

impl From<PhysicalRegister> for usize {
	fn from(value: PhysicalRegister) -> Self {
		value.index()
	}
}

#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[repr(transparent)]
#[serde(transparent)]
pub struct PhysicalRegisterSet {
	bits: [Bits; Self::LEN],
}

impl PhysicalRegisterSet {
	const BITS: usize = core::mem::size_of::<Bits>() * 8;
	const LEN: usize = PhysicalRegister::NUM_INDEX.div_ceil(Self::BITS);

	#[must_use]
	pub const fn empty() -> Self {
		Self {
			bits: [0; Self::LEN],
		}
	}

	const fn split_index(reg: PhysicalRegister) -> (usize, usize) {
		let index = reg.index();
		(index >> Self::BITS.ilog2(), index & (Self::BITS - 1))
	}

	#[must_use]
	pub const fn contains(self, reg: PhysicalRegister) -> bool {
		let (index, bit) = Self::split_index(reg);
		!matches!(self.bits[index] & (1 << bit), 0)
	}

	#[must_use]
	pub const fn with(self, reg: PhysicalRegister) -> Self {
		let (index, bit) = Self::split_index(reg);
		let mut out = self;
		out.bits[index] |= 1 << bit;
		out
	}

	pub const fn add(&mut self, reg: PhysicalRegister) {
		let (index, bit) = Self::split_index(reg);
		self.bits[index] |= 1 << bit;
	}

	pub const fn remove(&mut self, reg: PhysicalRegister) {
		let (index, bit) = Self::split_index(reg);
		self.bits[index] &= !(1 << bit);
	}

	pub fn union_from(&mut self, other: Self) {
		*self |= other;
	}

	pub fn intersect_from(&mut self, other: Self) {
		*self &= other;
	}

	#[must_use]
	pub fn invert(self) -> Self {
		let mut set = self.bits;
		for (i, bit) in set.iter_mut().enumerate() {
			*bit = !self.bits[i];
		}

		Self { bits: set }
	}

	#[must_use]
	pub const fn is_empty(self, class: RegisterClass) -> bool {
		matches!(self.bits[class as usize], 0)
	}

	#[must_use]
	pub const fn iter(self) -> PhysicalRegisterSetIter {
		PhysicalRegisterSetIter {
			set: self,
			index: 0,
		}
	}
}

impl BitAnd for PhysicalRegisterSet {
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output {
		let mut out = self;
		out.bitand_assign(rhs);
		out
	}
}

impl BitAndAssign for PhysicalRegisterSet {
	fn bitand_assign(&mut self, rhs: Self) {
		for i in 0..self.bits.len() {
			self.bits[i] &= rhs.bits[i];
		}
	}
}

impl BitOr for PhysicalRegisterSet {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		let mut out = self;
		out.bitor_assign(rhs);
		out
	}
}

impl BitOrAssign for PhysicalRegisterSet {
	fn bitor_assign(&mut self, rhs: Self) {
		for i in 0..self.bits.len() {
			self.bits[i] |= rhs.bits[i];
		}
	}
}

impl Display for PhysicalRegisterSet {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_char('{')?;
		for reg in *self {
			Display::fmt(&reg, f)?;
			f.write_str(", ")?;
		}

		f.write_char('}')
	}
}

impl FromIterator<PhysicalRegister> for PhysicalRegisterSet {
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = PhysicalRegister>,
	{
		let mut set = Self::default();
		for reg in iter {
			set.add(reg);
		}

		set
	}
}

impl IntoIterator for PhysicalRegisterSet {
	type IntoIter = PhysicalRegisterSetIter;
	type Item = PhysicalRegister;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub struct PhysicalRegisterSetIter {
	set: PhysicalRegisterSet,
	index: usize,
}

impl Iterator for PhysicalRegisterSetIter {
	type Item = PhysicalRegister;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let bits = self.set.bits.get_mut(self.index)?;
			if !matches!(bits, 0) {
				let bit = bits.trailing_zeros();
				*bits &= !(1 << bit);
				let index = bit as usize + self.index * PhysicalRegisterSet::BITS;
				break Some(PhysicalRegister::from_index(index));
			}
			self.index += 1;
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct VirtualRegister {
	bits: u32,
}

impl VirtualRegister {
	pub const MAX: usize = (1 << Self::MAX_BITS) - 1;
	pub const MAX_BITS: usize = 21;

	#[must_use]
	pub const fn new(virt_reg: usize, class: RegisterClass) -> Self {
		debug_assert!(virt_reg <= Self::MAX);
		Self {
			bits: ((virt_reg as u32) << 2) | (class as u8 as u32),
		}
	}

	#[must_use]
	pub const fn virtual_register(self) -> usize {
		(self.bits >> 2) as usize
	}

	#[must_use]
	pub const fn class(self) -> RegisterClass {
		match self.bits & 0b11 {
			0 => RegisterClass::Int,
			1 => RegisterClass::Float,
			2 => RegisterClass::Vector,
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub const fn bits(self) -> usize {
		self.bits as usize
	}

	#[must_use]
	pub const fn invalid() -> Self {
		Self::new(Self::MAX, RegisterClass::Int)
	}
}

impl Debug for VirtualRegister {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str("VirtualRegister(vreg = ")?;
		Display::fmt(&self.virtual_register(), f)?;
		f.write_str(", class = ")?;
		Debug::fmt(&self.class(), f)?;
		f.write_char(')')
	}
}

impl Display for VirtualRegister {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_char('v')?;
		Display::fmt(&self.virtual_register(), f)
	}
}

impl From<u32> for VirtualRegister {
	fn from(value: u32) -> Self {
		Self { bits: value }
	}
}

pub struct SpillSlot {
    bits: u32,
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize_repr, Deserialize_repr,
)]
#[repr(u8)]
pub enum RegisterClass {
	Int = 0,
	Float = 1,
	Vector = 2,
}

type Bits = u64;
