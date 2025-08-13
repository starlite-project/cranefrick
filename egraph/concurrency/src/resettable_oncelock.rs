use std::{cell::UnsafeCell, sync::Once};

pub struct ResettableOnceLock<T> {
	data: UnsafeCell<T>,
	update: Once,
}

impl<T> ResettableOnceLock<T> {
	pub const fn new(elt: T) -> Self {
		Self {
			data: UnsafeCell::new(elt),
			update: Once::new(),
		}
	}

	pub fn get(&self) -> Option<&T> {
		if self.update.is_completed() {
			unsafe { Some(&*self.data.get()) }
		} else {
			None
		}
	}

	pub fn get_or_update(&self, update: impl FnOnce(&mut T)) -> &T {
		if let Some(elt) = self.get() {
			return elt;
		}

		self.update.call_once_force(|_| {
			let elt = unsafe { &mut *self.data.get() };

			update(elt);
		});

		unsafe { &*self.data.get() }
	}

	pub const fn reset(&mut self) {
		self.update = Once::new();
	}
}

unsafe impl<T: Send> Send for ResettableOnceLock<T> {}
unsafe impl<T: Sync> Sync for ResettableOnceLock<T> {}

#[cfg(test)]
mod tests {
	use std::{
		sync::{
			Arc, Barrier,
			atomic::{AtomicUsize, Ordering},
		},
		thread,
	};

	use super::ResettableOnceLock;

	#[test]
	fn basic() {
		let lock = ResettableOnceLock::new(0);

		assert!(lock.get().is_none());

		let value = lock.get_or_update(|x| *x = 42);
		assert_eq!(*value, 42);

		assert_eq!(*lock.get().unwrap(), 42);

		let value = lock.get_or_update(|x| *x = 100);
		assert_eq!(*value, 42);
	}

	#[test]
	fn reset() {
		let mut lock = ResettableOnceLock::new(0);

		lock.get_or_update(|x| *x = 42);
		assert_eq!(*lock.get().unwrap(), 42);

		lock.reset();

		assert!(lock.get().is_none());

		let value = lock.get_or_update(|x| *x = 100);
		assert_eq!(*value, 100);

		assert_eq!(*lock.get().unwrap(), 100);
	}

	#[test]
	fn concurrent_readers() {
		let lock = Arc::new(ResettableOnceLock::new(0));
		let barrier = Arc::new(Barrier::new(10));

		lock.get_or_update(|x| *x = 42);

		let threads = (0..10)
			.map(|_| {
				let lock = lock.clone();
				let barrier = barrier.clone();
				thread::spawn(move || {
					barrier.wait();
					let value = lock.get().unwrap();

					assert_eq!(*value, 42);
				})
			})
			.collect::<Vec<_>>();

		for t in threads {
			t.join().unwrap();
		}
	}

	#[test]
	fn concurrent_get_or_update() {
		let lock = Arc::new(ResettableOnceLock::new(0));
		let counter = Arc::new(AtomicUsize::new(0));
		let barrier = Arc::new(Barrier::new(10));

		let threads = (0..10)
			.map(|_| {
				let lock = lock.clone();
				let counter = counter.clone();
				let barrier = barrier.clone();

				thread::spawn(move || {
					barrier.wait();

					let value = lock.get_or_update(|x| {
						counter.fetch_add(1, Ordering::SeqCst);
						*x = 42;
					});

					assert_eq!(*value, 42);
				})
			})
			.collect::<Vec<_>>();

		for t in threads {
			t.join().unwrap();
		}

		assert_eq!(counter.load(Ordering::SeqCst), 1);
		assert_eq!(*lock.get().unwrap(), 42);
	}

	#[test]
	fn update_mutability() {
		#[derive(Debug, PartialEq, Eq)]
		struct TestStruct {
			value: i32,
			updated: bool,
		}

		let lock = ResettableOnceLock::new(TestStruct {
			value: 0,
			updated: false,
		});

		lock.get_or_update(|data| {
			data.value = 100;
			data.updated = true;
		});

		let result = lock.get().unwrap();

		assert_eq!(result.value, 100);
		assert!(result.updated);
	}

	#[test]
	fn multiple_resets() {
		let mut lock = ResettableOnceLock::new(0);

		for i in 1..=5 {
			lock.get_or_update(|x| *x = i * 10);
			assert_eq!(*lock.get().unwrap(), i * 10);
			lock.reset();
			assert!(lock.get().is_none());
		}
	}

	#[test]
	fn is_sendable() {
		let lock = ResettableOnceLock::new(42);

		let handle = thread::spawn(move || {
			lock.get_or_update(|x| *x += 1);
			*lock.get().unwrap()
		});

		let result = handle.join().unwrap();
		assert_eq!(result, 43);
	}
}
