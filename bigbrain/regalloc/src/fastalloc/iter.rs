use core::{ops::Index, slice::SliceIndex};

use crate::{Operand, OperandConstraint, OperandType};

#[repr(transparent)]
pub struct Operands<'a>(pub &'a [Operand]);

impl<'a> Operands<'a> {
	pub const fn new(operands: &'a [Operand]) -> Self {
		Self(operands)
	}

	pub fn matches<F>(&self, predicate: F) -> impl Iterator<Item = (usize, Operand)> + 'a
	where
		F: Fn(Operand) -> bool + 'a,
	{
		self.0
			.iter()
			.copied()
			.enumerate()
			.filter(move |(.., op)| predicate(*op))
	}

	pub fn def_ops(&self) -> impl Iterator<Item = (usize, Operand)> + 'a {
		self.matches(|op| matches!(op.ty(), OperandType::Def))
	}

	pub fn use_ops(&self) -> impl Iterator<Item = (usize, Operand)> + 'a {
		self.matches(|op| matches!(op.ty(), OperandType::Use))
	}

	pub fn reuse(&self) -> impl Iterator<Item = (usize, Operand)> + 'a {
		self.matches(|op| matches!(op.constraint(), OperandConstraint::Reuse(..)))
	}

	pub fn fixed(&self) -> impl Iterator<Item = (usize, Operand)> + 'a {
		self.matches(|op| matches!(op.constraint(), OperandConstraint::FixedRegister(..)))
	}
}

impl<T> Index<T> for Operands<'_>
where
	T: SliceIndex<[Operand]>,
{
	type Output = T::Output;

	fn index(&self, index: T) -> &Self::Output {
		self.0.index(index)
	}
}
