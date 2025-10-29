use core::{
	iter::Step,
	ops::{Bound, RangeBounds},
};

pub trait ContainsRange<T> {
	fn contains_range<R>(&self, other: &R) -> bool
	where
		R: RangeBounds<T>;
}

impl<T: Step, R> ContainsRange<T> for R
where
	R: RangeBounds<T>,
{
	fn contains_range<R2>(&self, other: &R2) -> bool
	where
		R2: RangeBounds<T>,
	{
		check(self.start_bound(), other.start_bound(), false)
			&& check(other.end_bound(), self.end_bound(), true)
	}
}

fn check<T: Step>(left: Bound<&T>, right: Bound<&T>, end: bool) -> bool {
	match (left, right, end) {
		(Bound::Unbounded, _, false) | (.., Bound::Unbounded, true) => true,
		(.., Bound::Unbounded, false) | (Bound::Unbounded, _, true) => false,
		(Bound::Included(l), Bound::Included(r), ..)
		| (Bound::Excluded(l), Bound::Excluded(r), ..) => l <= r,
		(Bound::Excluded(l), Bound::Included(r), false)
		| (Bound::Included(l), Bound::Excluded(r), true) => l < r,
		(Bound::Included(l), Bound::Excluded(r), false)
		| (Bound::Excluded(l), Bound::Included(r), true) => l <= r || l == &Step::forward(r.clone(), 1),
	}
}

#[cfg(test)]
mod tests {
	use super::ContainsRange as _;

	#[test]
	fn basic() {
		let a = 0..6;
		let b = 1..=5;

		assert!(a.contains_range(&b));
	}
}
