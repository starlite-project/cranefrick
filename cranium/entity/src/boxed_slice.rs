use alloc::boxed::Box;
use core::{
	marker::PhantomData,
	ops::{Index, IndexMut},
	slice,
};

use super::{EntityRef, Iter, IterMut, Keys};

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct BoxedSlice<K: EntityRef, V> {
	values: Box<[V]>,
	marker: PhantomData<K>,
}

impl<K: EntityRef, V> BoxedSlice<K, V> {
	pub unsafe fn from_raw(raw: *mut [V]) -> Self {
		Self {
			values: unsafe { Box::from_raw(raw) },
			marker: PhantomData,
		}
	}

	#[must_use]
	pub fn len(&self) -> usize {
		self.values.len()
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.values.is_empty()
	}

	pub fn is_valid(&self, key: K) -> bool {
		key.index() < self.len()
	}

	pub fn get(&self, key: K) -> Option<&V> {
		self.values.get(key.index())
	}

	pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
		self.values.get_mut(key.index())
	}

	#[must_use]
	pub fn last(&self) -> Option<&V> {
		self.values.last()
	}

	#[must_use]
	pub fn keys(&self) -> Keys<K> {
		Keys::new(self.len())
	}

	pub fn values(&self) -> slice::Iter<'_, V> {
		self.values.iter()
	}

	pub fn values_mut(&mut self) -> slice::IterMut<'_, V> {
		self.values.iter_mut()
	}

	#[must_use]
	pub fn iter(&self) -> Iter<'_, K, V> {
		Iter::new(&self.values)
	}

	pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
		IterMut::new(&mut self.values)
	}
}

impl<K: EntityRef, V> Index<K> for BoxedSlice<K, V> {
	type Output = V;

	fn index(&self, index: K) -> &Self::Output {
		&self.values[index.index()]
	}
}

impl<K: EntityRef, V> IndexMut<K> for BoxedSlice<K, V> {
	fn index_mut(&mut self, index: K) -> &mut Self::Output {
		&mut self.values[index.index()]
	}
}

impl<'a, K: EntityRef, V> IntoIterator for &'a BoxedSlice<K, V> {
	type IntoIter = Iter<'a, K, V>;
	type Item = (K, &'a V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a, K: EntityRef, V> IntoIterator for &'a mut BoxedSlice<K, V> {
	type IntoIter = IterMut<'a, K, V>;
	type Item = (K, &'a mut V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}
