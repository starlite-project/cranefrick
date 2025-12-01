use core::iter::FusedIterator;

#[repr(transparent)]
pub struct WindowsN<'a, T: 'a, const N: usize> {
	slice: &'a [T],
}

impl<'a, T: 'a, const N: usize> WindowsN<'a, T, N> {
	const _LENGTH_CHECK: () = const { assert!(N != 0) };

	pub(super) const fn new(slice: &'a [T]) -> Self {
		Self { slice }
	}
}

impl<'a, T: 'a, const N: usize> Clone for WindowsN<'a, T, N> {
	fn clone(&self) -> Self {
		Self { slice: self.slice }
	}
}

impl<'a, T: 'a, const N: usize> DoubleEndedIterator for WindowsN<'a, T, N> {
	fn next_back(&mut self) -> Option<Self::Item> {
		if N > self.slice.len() {
			None
		} else {
			let ret = Some((&self.slice[self.slice.len() - N..]).try_into().ok()?);
			self.slice = &self.slice[..self.slice.len() - 1];
			ret
		}
	}

	fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
		let (end, overflow) = self.slice.len().overflowing_sub(n);
		if end < N || overflow {
			self.slice = &self.slice[..0];
			None
		} else {
			let ret = (&self.slice[end - N..end]).try_into().ok()?;
			self.slice = &self.slice[..end - 1];
			Some(ret)
		}
	}
}

impl<'a, T: 'a, const N: usize> ExactSizeIterator for WindowsN<'a, T, N> {}

impl<'a, T: 'a, const N: usize> FusedIterator for WindowsN<'a, T, N> {}

impl<'a, T: 'a, const N: usize> Iterator for WindowsN<'a, T, N> {
	type Item = &'a [T; N];

	fn next(&mut self) -> Option<Self::Item> {
		if N > self.slice.len() {
			None
		} else {
			let ret = Some((&self.slice[..N]).try_into().ok()?);
			self.slice = &self.slice[1..];
			ret
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		if N > self.slice.len() {
			(0, Some(0))
		} else {
			let size = self.slice.len() - N + 1;
			(size, Some(size))
		}
	}

	fn count(self) -> usize {
		self.len()
	}

	fn nth(&mut self, n: usize) -> Option<Self::Item> {
		if let Some(rest) = self.slice.get(n..)
			&& let Some(nth) = rest.get(..N)
		{
			self.slice = &rest[1..];
			Some(nth.try_into().ok()?)
		} else {
			self.slice = &self.slice[..0];
			None
		}
	}

	fn last(self) -> Option<Self::Item> {
		if N > self.slice.len() {
			None
		} else {
			let start = self.slice.len() - N;
			Some((&self.slice[start..]).try_into().ok()?)
		}
	}
}
