use core::{
	mem::{self, MaybeUninit},
	ops::{Deref, DerefMut},
};

pub struct RuntimeArray<T, const N: usize>(Option<[T; N]>);

impl<T, const N: usize> RuntimeArray<T, N> {
	#[inline]
	pub fn into_array(self) -> Option<[T; N]> {
		self.0
	}
}

impl<T, const N: usize> Deref for RuntimeArray<T, N> {
	type Target = Option<[T; N]>;

	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T, const N: usize> DerefMut for RuntimeArray<T, N> {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<T, const N: usize> From<[T; N]> for RuntimeArray<T, N> {
	#[inline]
	fn from(value: [T; N]) -> Self {
		Self(Some(value))
	}
}

impl<T, const N: usize> FromIterator<T> for RuntimeArray<T, N> {
	fn from_iter<V: IntoIterator<Item = T>>(iter: V) -> Self {
		let mut count = 0usize;
		let mut uninit_array: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };

		for value in iter {
			if count >= N {
				if mem::needs_drop::<T>() {
					unsafe { uninit_array[..].assume_init_drop() };
				}

				return Self(None);
			}

			uninit_array[count].write(value);

			count += 1;
		}

		if count == N {
			Self(Some(unsafe { mem::transmute_copy(&uninit_array) }))
		} else {
			if mem::needs_drop::<T>() {
				unsafe {
					uninit_array[..count].assume_init_drop();
				}
			}

			Self(None)
		}
	}
}

#[cfg(test)]
mod tests {
	extern crate alloc;

	use alloc::boxed::Box;

	use super::RuntimeArray;

	#[test]
	fn it_works() {
		let orig_array = alloc::vec![1u32, 2, 3];

		let arr = orig_array
			.clone()
			.into_iter()
			.collect::<RuntimeArray<_, 1>>();

		assert!(arr.is_none());

		let arr = orig_array
			.clone()
			.into_iter()
			.collect::<RuntimeArray<_, 2>>();

		assert!(arr.is_none());

		let arr = orig_array
			.clone()
			.into_iter()
			.collect::<RuntimeArray<_, 3>>();

		assert!(arr.is_some());

		let arr = orig_array
			.clone()
			.into_iter()
			.collect::<RuntimeArray<_, 4>>();

		assert!(arr.is_none());

		let arr = orig_array.into_iter().collect::<RuntimeArray<_, 5>>();

		assert!(arr.is_none());
	}

	#[test]
	fn drops_properly() {
		let orig_array = alloc::vec![Box::new(1u32), Box::new(2), Box::new(3)];

		let arr = orig_array
			.clone()
			.into_iter()
			.collect::<RuntimeArray<_, 1>>();

		assert!(arr.is_none());

		let arr = orig_array
			.clone()
			.into_iter()
			.collect::<RuntimeArray<_, 3>>();

		assert!(arr.is_some());

		let arr = orig_array.into_iter().collect::<RuntimeArray<_, 5>>();

		assert!(arr.is_none());
	}
}
