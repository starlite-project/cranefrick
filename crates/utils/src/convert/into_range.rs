use core::{
	iter::Step,
	ops::{Range, RangeInclusive},
};

pub trait IntoRange<T> {
	fn into_range(self) -> Range<T>;
}

impl<T: Step> IntoRange<T> for Range<T> {
	fn into_range(self) -> Self {
		self
	}
}

impl<T: Step> IntoRange<T> for RangeInclusive<T> {
	fn into_range(self) -> Range<T> {
		let start = self.start().clone();
		let end = T::forward(self.end().clone(), 1);

		start..end
	}
}

pub trait IntoRangeInclusive<T> {
	fn into_range_inclusive(self) -> RangeInclusive<T>;
}

impl<T> IntoRangeInclusive<T> for RangeInclusive<T> {
	fn into_range_inclusive(self) -> Self {
		self
	}
}

impl<T: Step> IntoRangeInclusive<T> for Range<T> {
	fn into_range_inclusive(self) -> RangeInclusive<T> {
		let Self { start, end } = self;
		let end = T::backward(end, 1);

		start..=end
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn into_range() {
		let a = 1..4;
		assert_eq!(a.clone().into_range(), a);

		let b = 1..=3;
		assert_eq!(b.into_range(), a);
	}

	#[test]
	fn into_range_inclusive() {
		let a = 1..=1;
		assert_eq!(a.clone().into_range_inclusive(), a);

		let b = 1..2;
		assert_eq!(b.into_range_inclusive(), a);
	}
}
