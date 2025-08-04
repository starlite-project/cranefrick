use std::{
	collections::{HashMap, HashSet, hash_map::Entry},
	hash::Hash,
	ops::Index,
};

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct StableSet<T>(HashSet<T>);

impl<T> StableSet<T> {
	pub(crate) fn new() -> Self {
		Self(HashSet::new())
	}
}

impl<T> StableSet<T>
where
	T: Eq + Hash,
{
	pub fn insert(&mut self, value: T) -> bool {
		self.0.insert(value)
	}

	pub fn contains(&self, value: &T) -> bool {
		self.0.contains(value)
	}
}

impl<T> Default for StableSet<T> {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct StableMap<K, V>(HashMap<K, V>);

impl<K, V> StableMap<K, V> {
	pub(crate) fn new() -> Self {
		Self(HashMap::new())
	}

	pub(crate) fn len(&self) -> usize {
		self.0.len()
	}
}

impl<K, V> StableMap<K, V>
where
	K: Eq + Hash,
{
	pub(crate) fn insert(&mut self, key: K, value: V) -> Option<V> {
		self.0.insert(key, value)
	}

	pub(crate) fn contains_key(&self, key: &K) -> bool {
		self.0.contains_key(key)
	}

	pub(crate) fn get(&self, key: &K) -> Option<&V> {
		self.0.get(key)
	}

	pub(crate) fn entry(&mut self, key: K) -> Entry<'_, K, V> {
		self.0.entry(key)
	}
}

impl<K, V> Default for StableMap<K, V> {
	fn default() -> Self {
		Self::new()
	}
}

impl<K, V> FromIterator<(K, V)> for StableMap<K, V>
where
	K: Eq + Hash,
{
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = (K, V)>,
	{
		Self(HashMap::from_iter(iter))
	}
}

impl<K, V> Index<&K> for StableMap<K, V>
where
	K: Eq + Hash,
{
	type Output = V;

	fn index(&self, index: &K) -> &Self::Output {
		self.0.index(index)
	}
}
