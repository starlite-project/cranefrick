use alloc::vec::Vec;
use core::{
	fmt::{Debug, Formatter, Result as FmtResult},
	mem, slice,
};

use serde::{Deserialize, Serialize};

use super::{EntityRef, SecondaryMap};

#[derive(Serialize, Deserialize)]
pub struct SparseMap<K: EntityRef, V>
where
	V: SparseMapValue<K>,
{
	sparse: SecondaryMap<K, u32>,
	dense: Vec<V>,
}

impl<K: EntityRef, V> SparseMap<K, V>
where
	V: SparseMapValue<K>,
{
	#[must_use]
	pub fn new() -> Self {
		Self {
			sparse: SecondaryMap::new(),
			dense: Vec::new(),
		}
	}

	#[must_use]
	pub const fn len(&self) -> usize {
		self.dense.len()
	}

	#[must_use]
	pub const fn is_empty(&self) -> bool {
		self.dense.is_empty()
	}

	pub fn clear(&mut self) {
		self.dense.clear();
	}

	pub fn get(&self, key: K) -> Option<&V> {
		let idx = self.sparse.get(key).copied()?;

		let entry = self.dense.get(idx as usize)?;

		(entry.key() == key).then_some(entry)
	}

	pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
		let idx = self.sparse.get(key).copied()?;

		let entry = self.dense.get_mut(idx as usize)?;

		(entry.key() == key).then_some(entry)
	}

	fn index(&self, key: K) -> Option<usize> {
		let idx = self.sparse.get(key).copied()? as usize;

		let entry = self.dense.get(idx)?;

		(entry.key() == key).then_some(idx)
	}

	pub fn contains_key(&self, key: K) -> bool {
		self.get(key).is_some()
	}

	pub fn insert(&mut self, value: V) -> Option<V> {
		let key = value.key();

		if let Some(entry) = self.get_mut(key) {
			return Some(mem::replace(entry, value));
		}

		let idx = self.len();
		debug_assert!(u32::try_from(idx).is_ok(), "SparseMap overflow");
		self.dense.push(value);
		self.sparse[key] = idx as u32;

		None
	}

	pub fn remove(&mut self, key: K) -> Option<V> {
		let idx = self.index(key)?;

		let back = self.pop().unwrap();

		if idx == self.dense.len() {
			return Some(back);
		}

		self.sparse[back.key()] = idx as u32;
		Some(mem::replace(&mut self.dense[idx], back))
	}

	pub fn pop(&mut self) -> Option<V> {
		self.dense.pop()
	}

	pub fn values(&self) -> slice::Iter<'_, V> {
		self.dense.iter()
	}

	#[must_use]
	pub const fn as_slice(&self) -> &[V] {
		self.dense.as_slice()
	}
}

impl<K, V> Debug for SparseMap<K, V>
where
	K: Debug + EntityRef,
	V: Debug + SparseMapValue<K>,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_map()
			.entries(self.values().map(|v| (v.key(), v)))
			.finish()
	}
}

impl<K: EntityRef, V> Default for SparseMap<K, V>
where
	V: SparseMapValue<K>,
{
	fn default() -> Self {
		Self::new()
	}
}

#[allow(clippy::into_iter_without_iter)]
impl<'a, K: EntityRef, V> IntoIterator for &'a SparseMap<K, V>
where
	V: SparseMapValue<K>,
{
	type IntoIter = slice::Iter<'a, V>;
	type Item = &'a V;

	fn into_iter(self) -> Self::IntoIter {
		self.values()
	}
}

pub trait SparseMapValue<K> {
	fn key(&self) -> K;
}

impl<T: EntityRef> SparseMapValue<T> for T {
	fn key(&self) -> T {
		*self
	}
}

pub type SparseSet<T> = SparseMap<T, T>;
