use alloc::{boxed::Box, vec::Vec};
use core::{
	fmt::{Debug, Formatter, Result as FmtResult},
	marker::PhantomData,
	ops::{Index, IndexMut},
	slice,
};

use serde::{Deserialize, Serialize};

use super::{BoxedSlice, EntityRef, IntoIter, Iter, IterMut, Keys};

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PrimaryMap<K: EntityRef, V> {
	values: Vec<V>,
	marker: PhantomData<K>,
}

impl<K: EntityRef, V> PrimaryMap<K, V> {
	#[must_use]
	pub const fn new() -> Self {
		Self {
			values: Vec::new(),
			marker: PhantomData,
		}
	}

	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			values: Vec::with_capacity(capacity),
			marker: PhantomData,
		}
	}

	pub fn is_valid(&self, key: K) -> bool {
		key.index() < self.values.len()
	}

	pub fn get(&self, key: K) -> Option<&V> {
		self.values.get(key.index())
	}

	pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
		self.values.get_mut(key.index())
	}

	#[must_use]
	pub const fn is_empty(&self) -> bool {
		self.values.is_empty()
	}

	#[must_use]
	pub const fn len(&self) -> usize {
		self.values.len()
	}

	#[must_use]
	pub const fn keys(&self) -> Keys<K> {
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

	pub fn clear(&mut self) {
		self.values.clear();
	}

	#[must_use]
	pub fn next_key(&self) -> K {
		K::new(self.len())
	}

	pub fn push(&mut self, v: V) -> K {
		let k = self.next_key();
		self.values.push(v);
		k
	}

	#[must_use]
	pub fn last(&self) -> Option<(K, &V)> {
		let len = self.len();
		let last = self.values.last()?;
		Some((K::new(len - 1), last))
	}

	pub fn last_mut(&mut self) -> Option<(K, &mut V)> {
		let len = self.len();
		let last = self.values.last_mut()?;
		Some((K::new(len - 1), last))
	}

	pub fn reserve(&mut self, additional: usize) {
		self.values.reserve(additional);
	}

	pub fn reserve_exact(&mut self, additional: usize) {
		self.values.reserve_exact(additional);
	}

	pub fn shrink_to(&mut self, to: usize) {
		self.values.shrink_to(to);
	}

	pub fn shrink_to_fit(&mut self) {
		self.values.shrink_to_fit();
	}

	#[must_use]
	pub fn into_boxed_slice(self) -> BoxedSlice<K, V> {
		unsafe {
			BoxedSlice::<K, V>::from_raw(Box::<[V]>::into_raw(self.values.into_boxed_slice()))
		}
	}

	pub fn get_disjoint_mut<const N: usize>(
		&mut self,
		indices: [K; N],
	) -> Result<[&mut V; N], slice::GetDisjointMutError> {
		self.values.get_disjoint_mut(indices.map(EntityRef::index))
	}

	pub fn binary_search_values_by_key<'a, B: Ord>(
		&'a self,
		b: &B,
		f: impl FnMut(&'a V) -> B,
	) -> Result<K, K> {
		self.values
			.binary_search_by_key(b, f)
			.map(EntityRef::new)
			.map_err(EntityRef::new)
	}

	pub fn get_raw_mut(&mut self, key: K) -> Option<*mut V> {
		if key.index() < self.len() {
			unsafe { Some(self.values.as_mut_ptr().add(key.index())) }
		} else {
			None
		}
	}
}

impl<K, V: Debug> Debug for PrimaryMap<K, V>
where
	K: Debug + EntityRef,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let mut state = f.debug_struct("PrimaryMap");

		for (k, v) in self {
			state.field(&alloc::format!("{k:?}"), v);
		}

		state.finish()
	}
}

impl<K: EntityRef, V> Default for PrimaryMap<K, V> {
	fn default() -> Self {
		Self::new()
	}
}

impl<K: EntityRef, V> FromIterator<V> for PrimaryMap<K, V> {
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = V>,
	{
		Self {
			values: Vec::from_iter(iter),
			marker: PhantomData,
		}
	}
}

impl<K: EntityRef, V> Index<K> for PrimaryMap<K, V> {
	type Output = V;

	fn index(&self, index: K) -> &Self::Output {
		&self.values[index.index()]
	}
}

impl<K: EntityRef, V> IndexMut<K> for PrimaryMap<K, V> {
	fn index_mut(&mut self, index: K) -> &mut Self::Output {
		&mut self.values[index.index()]
	}
}

impl<'a, K: EntityRef, V> IntoIterator for &'a PrimaryMap<K, V> {
	type IntoIter = Iter<'a, K, V>;
	type Item = (K, &'a V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a, K: EntityRef, V> IntoIterator for &'a mut PrimaryMap<K, V> {
	type IntoIter = IterMut<'a, K, V>;
	type Item = (K, &'a mut V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}

impl<K: EntityRef, V> IntoIterator for PrimaryMap<K, V> {
	type IntoIter = IntoIter<K, V>;
	type Item = (K, V);

	fn into_iter(self) -> Self::IntoIter {
		IntoIter::new(self.values)
	}
}
