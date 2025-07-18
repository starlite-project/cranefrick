use alloc::{
	borrow::{Cow, ToOwned},
	vec::Vec,
};

pub trait InsertOrPush<T> {
	fn insert_or_push(&mut self, index: usize, value: T);
}

impl<T> InsertOrPush<T> for Vec<T> {
	fn insert_or_push(&mut self, index: usize, value: T) {
		if index >= self.len() {
			self.push(value);
		} else {
			self.insert(index, value);
		}
	}
}

impl<T> InsertOrPush<T> for Cow<'_, [T]>
where
	[T]: ToOwned,
	<[T] as ToOwned>::Owned: InsertOrPush<T>,
{
	fn insert_or_push(&mut self, index: usize, value: T) {
		self.to_mut().insert_or_push(index, value);
	}
}

#[cfg(test)]
mod tests {
	use alloc::{borrow::Cow, vec};

	use super::InsertOrPush as _;

	#[test]
	fn vec_inserts() {
		let mut vec = vec![0u8, 1, 2, 3, 4, 5];

		vec.insert_or_push(2, 6);

		assert_eq!(vec, [0, 1, 6, 2, 3, 4, 5]);
	}

	#[test]
	fn vec_pushes() {
		let mut vec = vec![0u8, 1];

		vec.insert_or_push(2, 6);

		assert_eq!(vec, [0, 1, 6]);
	}

	#[test]
	fn cow_inserts() {
		let mut cow = Cow::Borrowed(([0u8, 1, 2, 3, 4, 5]).as_slice());

		cow.insert_or_push(2, 6);

		assert_eq!(cow, Cow::<[u8]>::Owned(vec![0, 1, 6, 2, 3, 4, 5]));
	}

	#[test]
	fn cow_pushes() {
		let mut cow = Cow::Borrowed(([0u8, 1]).as_slice());

		cow.insert_or_push(2, 6);

		assert_eq!(cow, Cow::<[u8]>::Owned(vec![0, 1, 6]));
	}
}
