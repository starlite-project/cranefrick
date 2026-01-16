use core::{
	mem::MaybeUninit,
	ops::{Deref, DerefMut},
};

pub struct RuntimeArray<T, const N: usize>(Option<[T; N]>);

impl<T, const N: usize> RuntimeArray<T, N> {
	pub fn into_array(self) -> Option<[T; N]> {
		self.0
	}
}

impl<T, const N: usize> Deref for RuntimeArray<T, N> {
	type Target = Option<[T; N]>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T, const N: usize> DerefMut for RuntimeArray<T, N> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<T, const N: usize> From<[T; N]> for RuntimeArray<T, N> {
	fn from(value: [T; N]) -> Self {
		Self(Some(value))
	}
}

impl<T, const N: usize> FromIterator<T> for RuntimeArray<T, N> {
	fn from_iter<V: IntoIterator<Item = T>>(iter: V) -> Self {
		let mut count = 0usize;
		let mut uninit_array = MaybeUninit::<[T; N]>::uninit();

		for value in iter {
			if count >= N {
				return Self(None);
			}

			unsafe {
				uninit_array
					.as_mut_ptr()
					.cast::<T>()
					.add(count)
					.write(value);
			}

			count += 1;
		}

		if count == N {
			Self(Some(unsafe { MaybeUninit::assume_init(uninit_array) }))
		} else {
			Self(None)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::RuntimeArray;

	#[test]
	fn it_works() {
		let orig_array = [1, 2, 3];

		let arr = orig_array.into_iter().collect::<RuntimeArray<_, 1>>();

		assert!(arr.is_none());

		let arr = orig_array.into_iter().collect::<RuntimeArray<_, 5>>();

		assert!(arr.is_none());

		let arr = orig_array.into_iter().collect::<RuntimeArray<_, 3>>();

		assert!(arr.is_some());
	}
}
