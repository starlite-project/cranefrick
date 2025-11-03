use core::iter::FusedIterator;

#[repr(transparent)]
pub struct WindowsN<'a, T: 'a, const N: usize> {
	v: &'a [T],
}

impl<'a, T: 'a, const N: usize> WindowsN<'a, T, N> {
	const fn new(slice: &'a [T]) -> Self {
		Self { v: slice }
	}
}

impl<'a, T: 'a, const N: usize> Clone for WindowsN<'a, T, N> {
	fn clone(&self) -> Self {
		Self { v: self.v }
	}
}

impl<'a, T: 'a, const N: usize> DoubleEndedIterator for WindowsN<'a, T, N> {
	fn next_back(&mut self) -> Option<Self::Item> {
		if N > self.v.len() {
			None
		} else {
			let ret = Some((&self.v[self.v.len() - N..]).try_into().ok()?);
			self.v = &self.v[..self.v.len() - 1];
			ret
		}
	}

	fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
		let (end, overflow) = self.v.len().overflowing_sub(n);
		if end < N || overflow {
			self.v = &self.v[..0];
			None
		} else {
			let ret = (&self.v[end - N..end]).try_into().ok()?;
			self.v = &self.v[..end - 1];
			Some(ret)
		}
	}
}

impl<'a, T: 'a, const N: usize> ExactSizeIterator for WindowsN<'a, T, N> {}

impl<'a, T: 'a, const N: usize> FusedIterator for WindowsN<'a, T, N> {}

impl<'a, T: 'a, const N: usize> Iterator for WindowsN<'a, T, N> {
	type Item = &'a [T; N];

	fn next(&mut self) -> Option<Self::Item> {
		if N > self.v.len() {
			None
		} else {
			let ret = Some((&self.v[..N]).try_into().ok()?);
			self.v = &self.v[1..];
			ret
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		if N > self.v.len() {
			(0, Some(0))
		} else {
			let size = self.v.len() - N + 1;
			(size, Some(size))
		}
	}

	fn count(self) -> usize {
		self.len()
	}

	fn nth(&mut self, n: usize) -> Option<Self::Item> {
		if let Some(rest) = self.v.get(n..)
			&& let Some(nth) = rest.get(..N)
		{
			self.v = &rest[1..];
			Some(nth.try_into().ok()?)
		} else {
			self.v = &self.v[..0];
			None
		}
	}

	fn last(self) -> Option<Self::Item> {
		if N > self.v.len() {
			None
		} else {
			let start = self.v.len() - N;
			Some((&self.v[start..]).try_into().ok()?)
		}
	}
}

pub trait SliceExt<T> {
	fn windows_n<const N: usize>(&self) -> WindowsN<'_, T, N>;
}

impl<T> SliceExt<T> for [T] {
	fn windows_n<const N: usize>(&self) -> WindowsN<'_, T, N> {
		WindowsN::new(self)
	}
}
