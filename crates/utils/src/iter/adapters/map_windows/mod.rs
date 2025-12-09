mod array_internals;

use core::mem::{self, MaybeUninit};

#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct MapWindows<I: Iterator, F, const N: usize> {
	mapper: F,
	inner: MapWindowsInner<I, N>,
}

impl<I: Iterator, F, const N: usize> MapWindows<I, F, N> {
	pub(crate) const fn new(iter: I, mapper: F) -> Self {
		assert!(
			N != 0,
			"array in `Iterator::map_windows` must contain more than 0 elements"
		);

		if matches!(mem::size_of::<I::Item>(), 0) {
			assert!(
				N.checked_mul(2).is_some(),
				"array size of `Iterator::map_windows` is too large"
			);
		}

		Self {
			inner: MapWindowsInner::new(iter),
			mapper,
		}
	}
}

struct MapWindowsInner<I: Iterator, const N: usize> {
	iter: Option<I>,
	buffer: Option<Buffer<I::Item, N>>,
}

impl<I: Iterator, const N: usize> MapWindowsInner<I, N> {
	const fn new(iter: I) -> Self {
		Self {
			iter: Some(iter),
			buffer: None,
		}
	}
}

struct Buffer<T, const N: usize> {
	buffer: [[MaybeUninit<T>; N]; 2],
	start: usize,
}
