mod serde;

use alloc::vec::Vec;
use core::{
	cmp,
	fmt::{Debug, Formatter, Result as FmtResult},
	marker::PhantomData,
	ops::{Index, IndexMut},
	slice,
};

use super::{EntityRef, Iter, IterMut, Keys};

#[allow(clippy::derived_hash_with_manual_eq)]
#[derive(Clone, Hash)]
pub struct SecondaryMap<K: EntityRef, V: Clone> {
	values: Vec<V>,
	default: V,
	marker: PhantomData<K>,
}

impl<K: EntityRef, V: Clone> SecondaryMap<K, V> {
	#[must_use]
	pub fn new() -> Self
	where
		V: Default,
	{
		Self {
			values: Vec::new(),
			default: V::default(),
			marker: PhantomData,
		}
	}

	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self
	where
		V: Default,
	{
		Self {
			values: Vec::with_capacity(capacity),
			default: V::default(),
			marker: PhantomData,
		}
	}

	pub const fn with_default(default: V) -> Self {
		Self {
			values: Vec::new(),
			default,
			marker: PhantomData,
		}
	}

	pub const fn capacity(&self) -> usize {
		self.values.capacity()
	}

	pub fn get(&self, key: K) -> Option<&V> {
		self.values.get(key.index())
	}

	pub const fn len(&self) -> usize {
		self.values.len()
	}

	pub const fn is_empty(&self) -> bool {
		self.values.is_empty()
	}

	pub fn clear(&mut self) {
		self.values.clear();
	}

	pub fn iter(&self) -> Iter<'_, K, V> {
		Iter::new(&self.values)
	}

	pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
		IterMut::new(&mut self.values)
	}

	pub const fn keys(&self) -> Keys<K> {
		Keys::new(self.len())
	}

	pub fn values(&self) -> slice::Iter<'_, V> {
		self.values.iter()
	}

	pub fn values_mut(&mut self) -> slice::IterMut<'_, V> {
		self.values.iter_mut()
	}

	pub fn resize(&mut self, n: usize) {
		self.values.resize(n, self.default.clone());
	}

	#[cold]
	fn resize_for_index_mut(&mut self, i: usize) -> &mut V {
		self.resize(i + 1);
		&mut self.values[i]
	}
}

impl<K, V> Debug for SecondaryMap<K, V>
where
	K: Debug + EntityRef,
	V: Clone + Debug,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_struct("SecondaryMap")
			.field("values", &self.values)
			.field("default", &self.default)
			.finish()
	}
}

impl<K: EntityRef, V> Default for SecondaryMap<K, V>
where
	V: Clone + Default,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<K: EntityRef, V> Eq for SecondaryMap<K, V> where V: Clone + Eq {}

impl<K: EntityRef, V> FromIterator<(K, V)> for SecondaryMap<K, V>
where
	V: Clone + Default,
{
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = (K, V)>,
	{
		let iter = iter.into_iter();
		let (min, max) = iter.size_hint();
		let cap = max.unwrap_or_else(|| min * 2);
		let mut map = Self::with_capacity(cap);
		for (k, v) in iter {
			map[k] = v;
		}

		map
	}
}

impl<K: EntityRef, V: Clone> Index<K> for SecondaryMap<K, V> {
	type Output = V;

	fn index(&self, index: K) -> &Self::Output {
		self.get(index).unwrap_or(&self.default)
	}
}

impl<K: EntityRef, V: Clone> IndexMut<K> for SecondaryMap<K, V> {
	fn index_mut(&mut self, index: K) -> &mut Self::Output {
		let i = index.index();
		if i >= self.len() {
			return self.resize_for_index_mut(i);
		}

		&mut self.values[i]
	}
}

impl<'a, K: EntityRef, V: Clone> IntoIterator for &'a SecondaryMap<K, V> {
	type IntoIter = Iter<'a, K, V>;
	type Item = (K, &'a V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a, K: EntityRef, V: Clone> IntoIterator for &'a mut SecondaryMap<K, V> {
	type IntoIter = IterMut<'a, K, V>;
	type Item = (K, &'a mut V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}

impl<K: EntityRef, V> PartialEq for SecondaryMap<K, V>
where
	V: Clone + PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		let min_size = cmp::min(self.len(), other.len());
		self.default == other.default
			&& self.values[..min_size] == other.values[..min_size]
			&& self.values[min_size..].iter().all(|e| *e == self.default)
			&& other.values[min_size..].iter().all(|e| *e == other.default)
	}
}
