use core::{
	fmt::Debug,
	iter::{DoubleEndedIterator, FusedIterator},
	ops::Range,
};

use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! define_index {
	($ix:ident, $storage:ident, $elem:ident) => {
		define_index!($ix);

		#[derive(Debug, Default, Clone)]
		pub struct $storage {
			storage: Vec<$elem>,
		}

		impl $storage {
			#[inline(always)]
			/// See `VecExt::preallocate`
			pub fn preallocate(&mut self, cap: usize) {
				use $crate::VecExt;
				self.storage.preallocate(cap);
			}

			#[inline(always)]
			pub fn len(&self) -> usize {
				self.storage.len()
			}

			#[inline(always)]
			pub fn iter(&self) -> impl Iterator<Item = &$elem> {
				self.storage.iter()
			}

			#[inline(always)]
			pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut $elem> {
				self.storage.iter_mut()
			}

			#[inline(always)]
			pub fn push(&mut self, value: $elem) -> $ix {
				let idx = $ix(self.storage.len() as u32);
				self.storage.push(value);
				idx
			}
		}

		impl core::ops::Index<$ix> for $storage {
			type Output = $elem;

			#[inline(always)]
			fn index(&self, i: $ix) -> &Self::Output {
				&self.storage[i.index()]
			}
		}

		impl core::ops::IndexMut<$ix> for $storage {
			#[inline(always)]
			fn index_mut(&mut self, i: $ix) -> &mut Self::Output {
				&mut self.storage[i.index()]
			}
		}

		impl<'a> IntoIterator for &'a $storage {
			type IntoIter = core::slice::Iter<'a, $elem>;
			type Item = &'a $elem;

			#[inline(always)]
			fn into_iter(self) -> Self::IntoIter {
				self.storage.iter()
			}
		}

		impl<'a> IntoIterator for &'a mut $storage {
			type IntoIter = core::slice::IterMut<'a, $elem>;
			type Item = &'a mut $elem;

			#[inline(always)]
			fn into_iter(self) -> Self::IntoIter {
				self.storage.iter_mut()
			}
		}
	};

	($ix:ident) => {
		#[derive(
			Debug,
			Clone,
			Copy,
			PartialEq,
			Eq,
			PartialOrd,
			Ord,
			Hash,
			::serde::Serialize,
			::serde::Deserialize,
		)]
		#[repr(transparent)]
		pub struct $ix(pub u32);
		impl $ix {
			#[inline(always)]
			#[must_use]
			pub const fn new(i: usize) -> Self {
				Self(i as u32)
			}

			#[must_use]
			#[inline(always)]
			pub fn index(self) -> usize {
				debug_assert!(self.is_valid());
				self.0 as usize
			}

			#[must_use]
			#[inline(always)]
			pub const fn invalid() -> Self {
				Self(u32::MAX)
			}

			#[must_use]
			#[inline(always)]
			pub fn is_invalid(self) -> bool {
				self == Self::invalid()
			}

			#[must_use]
			#[inline(always)]
			pub fn is_valid(self) -> bool {
				self != Self::invalid()
			}

			#[must_use]
			#[inline(always)]
			pub fn next(self) -> $ix {
				debug_assert!(self.is_valid());
				Self(self.0 + 1)
			}

			#[must_use]
			#[inline(always)]
			pub fn prev(self) -> $ix {
				debug_assert!(self.is_valid());
				Self(self.0 - 1)
			}

			#[must_use]
			#[inline(always)]
			pub const fn raw_u32(self) -> u32 {
				self.0
			}
		}

		impl $crate::index::ContainerIndex for $ix {}
	};
}

pub trait ContainerIndex: Clone + Copy + Debug + Eq {}

pub trait ContainerComparator {
	type Index: ContainerIndex;

	fn compare(&self, a: Self::Index, b: Self::Index) -> core::cmp::Ordering;
}

define_index!(Instruction);
define_index!(Block);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InstructionRange(Instruction, Instruction);

impl InstructionRange {
	#[must_use]
	pub fn new(from: Instruction, to: Instruction) -> Self {
		debug_assert!(from.index() <= to.index());
		Self(from, to)
	}

	#[must_use]
	pub fn first(self) -> Instruction {
		debug_assert!(!self.is_empty());
		self.0
	}

	#[must_use]
	pub fn last(self) -> Instruction {
		debug_assert!(!self.is_empty());
		self.1.prev()
	}

	#[must_use]
	pub fn rest(self) -> Self {
		debug_assert!(!self.is_empty());
		Self::new(self.0.next(), self.1)
	}

	#[must_use]
	pub fn len(self) -> usize {
		self.1.index() - self.0.index()
	}

	#[must_use]
	pub fn is_empty(self) -> bool {
		matches!(self.len(), 0)
	}

	#[must_use]
	pub fn iter(self) -> InstructionRangeIter {
		InstructionRangeIter {
			inner: (self.0.index()..self.1.index()),
		}
	}
}

impl IntoIterator for InstructionRange {
	type IntoIter = InstructionRangeIter;
	type Item = Instruction;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

#[repr(transparent)]
pub struct InstructionRangeIter {
	inner: Range<usize>,
}

impl DoubleEndedIterator for InstructionRangeIter {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.inner.next_back().map(Instruction::new)
	}
}

impl ExactSizeIterator for InstructionRangeIter {
	fn len(&self) -> usize {
		self.inner.len()
	}
}

impl FusedIterator for InstructionRangeIter {}

impl Iterator for InstructionRangeIter {
	type Item = Instruction;

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(Instruction::new)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}
}

#[cfg(test)]
mod tests {
	use alloc::vec::Vec;

	use super::*;

	#[test]
	fn inst_range() {
		let range = InstructionRange::new(Instruction::new(0), Instruction::new(0));
		assert_eq!(range.len(), 0);

		let range = InstructionRange::new(Instruction::new(0), Instruction::new(5));
		assert_eq!(range.first().index(), 0);
		assert_eq!(range.last().index(), 4);
		assert_eq!(range.len(), 5);
		assert_eq!(
			range.iter().collect::<Vec<_>>(),
			[
				Instruction::new(0),
				Instruction::new(1),
				Instruction::new(2),
				Instruction::new(3),
				Instruction::new(4)
			]
		);
	}
}
