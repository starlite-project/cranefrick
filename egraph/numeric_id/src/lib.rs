#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

use std::{
	fmt::{Debug, Formatter, Result as FmtResult},
	hash::Hash,
	iter::{Enumerate, FusedIterator},
	marker::PhantomData,
	ops::{self, RangeBounds},
	slice,
	sync::atomic::AtomicUsize,
	vec,
};

#[repr(transparent)]
pub struct DenseIdMap<K, V> {
	data: Vec<Option<V>>,
	marker: PhantomData<K>,
}

impl<K: NumericId, V> DenseIdMap<K, V> {
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self {
		let mut res = Self::new();
		res.reserve_space(K::from_index(capacity));
		res
	}

	#[must_use]
	pub const fn capacity(&self) -> usize {
		self.data.capacity()
	}

	#[must_use]
	pub const fn n_ids(&self) -> usize {
		self.data.len()
	}

	pub fn insert(&mut self, key: K, value: V) {
		self.reserve_space(key);
		self.data[key.index()] = Some(value);
	}

	#[must_use]
	pub fn next_id(&self) -> K {
		K::from_index(self.data.len())
	}

	pub fn push(&mut self, value: V) -> K {
		let res = self.next_id();
		self.data.push(Some(value));
		res
	}

	pub fn contains_key(&self, key: K) -> bool {
		self.data.get(key.index()).is_some_and(Option::is_some)
	}

	pub fn get(&self, key: K) -> Option<&V> {
		self.data.get(key.index()).and_then(|v| v.as_ref())
	}

	pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
		self.reserve_space(key);
		self.data.get_mut(key.index())?.as_mut()
	}

	pub fn unwrap_value(&mut self, key: K) -> V {
		self.take(key).unwrap()
	}

	pub fn get_or_insert(&mut self, key: K, f: impl FnOnce() -> V) -> &mut V {
		self.reserve_space(key);
		self.data[key.index()].get_or_insert_with(f)
	}

	#[must_use]
	pub fn raw(&self) -> &[Option<V>] {
		&self.data
	}

	pub fn take(&mut self, key: K) -> Option<V> {
		self.reserve_space(key);
		self.data.get_mut(key.index()).and_then(Option::take)
	}

	pub fn clear(&mut self) {
		self.data.clear();
	}

	pub fn reserve_space(&mut self, key: K) {
		let index = key.index();
		if index >= self.data.len() {
			self.data.resize_with(index + 1, || None);
		}
	}

	#[must_use]
	pub fn iter(&self) -> Iter<'_, K, V> {
		Iter {
			inner: Some(self.data.iter().enumerate()),
			marker: PhantomData,
		}
	}

	pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
		IterMut {
			inner: Some(self.data.iter_mut().enumerate()),
			marker: PhantomData,
		}
	}

	pub fn drain<R>(&mut self, range: R) -> Drain<'_, K, V>
	where
		R: RangeBounds<usize>,
	{
		Drain {
			inner: Some(self.data.drain(range).enumerate()),
			marker: PhantomData,
		}
	}
}

impl<K, V: Clone> Clone for DenseIdMap<K, V> {
	fn clone(&self) -> Self {
		Self {
			data: self.data.clone(),
			marker: PhantomData,
		}
	}
}

impl<K, V: Debug> Debug for DenseIdMap<K, V>
where
	K: Debug + NumericId,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let mut map = f.debug_map();
		map.entries(self.iter()).finish()
	}
}

impl<K, V> Default for DenseIdMap<K, V> {
	fn default() -> Self {
		Self {
			data: Vec::new(),
			marker: PhantomData,
		}
	}
}

impl<K, V: Eq> Eq for DenseIdMap<K, V> {}

impl<'a, K: NumericId, V> IntoIterator for &'a DenseIdMap<K, V> {
	type IntoIter = Iter<'a, K, V>;
	type Item = (K, &'a V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a, K: NumericId, V> IntoIterator for &'a mut DenseIdMap<K, V> {
	type IntoIter = IterMut<'a, K, V>;
	type Item = (K, &'a mut V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}

impl<K: NumericId, V> IntoIterator for DenseIdMap<K, V> {
	type IntoIter = IntoIter<K, V>;
	type Item = (K, V);

	fn into_iter(self) -> Self::IntoIter {
		IntoIter {
			inner: Some(self.data.into_iter().enumerate()),
			marker: PhantomData,
		}
	}
}

impl<K, V: PartialEq> PartialEq for DenseIdMap<K, V> {
	fn eq(&self, other: &Self) -> bool {
		PartialEq::eq(&self.data, &other.data)
	}
}

#[repr(transparent)]
pub struct Iter<'a, K, V> {
	inner: Option<Enumerate<slice::Iter<'a, Option<V>>>>,
	marker: PhantomData<K>,
}

impl<K: NumericId, V> FusedIterator for Iter<'_, K, V> {}

impl<'a, K: NumericId, V> Iterator for Iter<'a, K, V> {
	type Item = (K, &'a V);

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(inner) = self.inner.as_mut()
			&& let Some((i, Some(v))) = inner.next()
		{
			return Some((K::from_index(i), v));
		}

		self.inner = None;
		None
	}
}

#[repr(transparent)]
pub struct IterMut<'a, K, V> {
	inner: Option<Enumerate<slice::IterMut<'a, Option<V>>>>,
	marker: PhantomData<K>,
}

impl<K: NumericId, V> FusedIterator for IterMut<'_, K, V> {}

impl<'a, K: NumericId, V> Iterator for IterMut<'a, K, V> {
	type Item = (K, &'a mut V);

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(inner) = self.inner.as_mut()
			&& let Some((i, Some(v))) = inner.next()
		{
			return Some((K::from_index(i), v));
		}

		self.inner = None;
		None
	}
}

#[repr(transparent)]
pub struct IntoIter<K, V> {
	inner: Option<Enumerate<vec::IntoIter<Option<V>>>>,
	marker: PhantomData<K>,
}

impl<K: NumericId, V> FusedIterator for IntoIter<K, V> {}

impl<K: NumericId, V> Iterator for IntoIter<K, V> {
	type Item = (K, V);

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(inner) = self.inner.as_mut()
			&& let Some((i, Some(v))) = inner.next()
		{
			return Some((K::from_index(i), v));
		}

		self.inner = None;
		None
	}
}

pub struct Drain<'a, K, V> {
	inner: Option<Enumerate<vec::Drain<'a, Option<V>>>>,
	marker: PhantomData<K>,
}

impl<K: NumericId, V> FusedIterator for Drain<'_, K, V> {}

impl<K: NumericId, V> Iterator for Drain<'_, K, V> {
	type Item = (K, V);

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(inner) = self.inner.as_mut()
			&& let Some((i, Some(v))) = inner.next()
		{
			return Some((K::from_index(i), v));
		}

		self.inner = None;
		None
	}
}

pub trait NumericId: Copy + Eq + Hash + Ord + Send + Sync {
	type Repr;
	type Atomic;

	fn new(value: Self::Repr) -> Self;

	fn from_index(index: usize) -> Self;

	fn index(self) -> usize;

	fn repr(self) -> Self::Repr;

	#[must_use]
	fn increment(self) -> Self {
		Self::from_index(self.index() + 1)
	}
}

impl NumericId for usize {
	type Atomic = AtomicUsize;
	type Repr = Self;

	fn new(value: Self::Repr) -> Self {
		value
	}

	fn from_index(index: usize) -> Self {
		index
	}

	fn repr(self) -> Self::Repr {
		self
	}

	fn index(self) -> usize {
		self
	}

	fn increment(self) -> Self {
		self + 1
	}
}
