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
