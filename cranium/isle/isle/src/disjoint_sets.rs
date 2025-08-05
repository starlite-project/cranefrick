use std::{collections::HashMap, fmt::Debug, hash::Hash, mem};

#[derive(Debug, Default, Clone)]
#[repr(transparent)]
pub struct DisjointSets<T> {
	parent: HashMap<T, (T, u8)>,
}

impl<T> DisjointSets<T>
where
	T: Copy + Debug + Eq + Hash,
{
	pub fn find_mut(&mut self, mut x: T) -> Option<T> {
		while let Some(node) = self.parent.get(&x) {
			if node.0 == x {
				return Some(x);
			}

			let grandparent = self.parent[&node.0].0;
			self.parent.get_mut(&x).unwrap().0 = grandparent;
			x = grandparent;
		}

		None
	}

	pub fn find(&self, mut x: T) -> Option<T> {
		while let Some(node) = self.parent.get(&x) {
			if node.0 == x {
				return Some(x);
			}

			x = node.0;
		}

		None
	}

	pub fn merge(&mut self, x: T, y: T) {
		assert_ne!(x, y);

		let mut x = if let Some(x) = self.find_mut(x) {
			self.parent[&x]
		} else {
			self.parent.insert(x, (x, 0));
			(x, 0)
		};

		let mut y = if let Some(y) = self.find_mut(y) {
			self.parent[&y]
		} else {
			self.parent.insert(y, (y, 0));
			(y, 0)
		};

		if x == y {
			return;
		}

		if x.1 < y.1 {
			mem::swap(&mut x, &mut y);
		}

		self.parent.get_mut(&y.0).unwrap().0 = x.0;
		if x.1 == y.1 {
			let x_rank = &mut self.parent.get_mut(&x.0).unwrap().1;
			*x_rank = x_rank.saturating_add(1);
		}
	}

	pub fn in_same_set(&self, x: T, y: T) -> bool {
		let x = self.find(x);
		let y = self.find(y);
		x.zip(y).is_some_and(|(x, y)| x == y)
	}

	pub fn remove_set_of(&mut self, x: T) -> Vec<T>
	where
		T: Ord,
	{
		let mut set = Vec::new();

		if let Some(x) = self.find_mut(x) {
			set.extend(self.parent.keys().copied());

			set.retain(|&y| self.find_mut(y).unwrap() == x);
			for y in &set {
				self.parent.remove(y);
			}

			set.sort_unstable();
		}

		set
	}

	#[must_use]
	pub fn len(&self) -> usize {
		self.parent.len()
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.parent.is_empty()
	}
}
