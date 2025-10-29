mod change;
mod set;

use core::cmp::Ordering;

pub use self::{change::*, set::*};

#[derive(PartialEq, Eq)]
struct OffsetSorterKey(i32);

impl Ord for OffsetSorterKey {
	fn cmp(&self, other: &Self) -> Ordering {
		let lhs = self.0;
		let rhs = other.0;

		lhs.abs().cmp(&rhs.abs()).then_with(|| lhs.cmp(&rhs))
	}
}

impl PartialOrd for OffsetSorterKey {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
