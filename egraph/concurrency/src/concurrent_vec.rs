use std::{
	cell::UnsafeCell,
	mem::{self, MaybeUninit},
	ops::Deref,
	sync::{
		Mutex,
		atomic::{AtomicUsize, Ordering},
	},
};

use super::{MutexReader, ReadOptimizedLock};

pub struct ConcurrentVec<T> {
	data: ReadOptimizedLock<Vec<MaybeUninit<SyncUnsafeCell<T>>>>,
	head: AtomicUsize,
	write_lock: Mutex<()>,
}

impl<T> ConcurrentVec<T> {
	#[must_use]
	pub fn new() -> Self {
		Self::with_capacity(128)
	}

	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self {
		let capacity = capacity.next_power_of_two();
		Self {
			data: ReadOptimizedLock::new(Vec::with_capacity(capacity)),
			head: AtomicUsize::new(0),
			write_lock: Mutex::new(()),
		}
	}

	pub fn push(&self, item: T) -> usize {
		let _guard = self.write_lock.lock().unwrap();
		let index = self.head.load(Ordering::Acquire);
		self.push_at(item, index);
		self.head.store(index + 1, Ordering::Release);
		index
	}

	pub fn read(&self) -> impl Deref<Target = [T]> + '_ {
		let valid_prefix = self.head.load(Ordering::Acquire);
		let reader = self.data.read();

		ReadHandle {
			valid_prefix,
			reader,
		}
	}

	fn push_at(&self, item: T, index: usize) {
		let handle = self.data.read();
		if let Some(slot) = handle.get(index) {
			unsafe { ((*slot.as_ptr()).0.get()).write(item) };
			return;
		}

		mem::drop(handle);
		let mut writer = self.data.lock();
		if index >= writer.len() {
			writer.resize_with((index + 1).next_power_of_two(), MaybeUninit::uninit);
		}

		mem::drop(writer);
		self.push_at(item, index);
	}
}

impl<T> Default for ConcurrentVec<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T> Drop for ConcurrentVec<T> {
	fn drop(&mut self) {
		let mut writer = self.data.lock();
		let len = self.head.load(Ordering::Acquire);
		if mem::needs_drop::<T>() {
			for i in 0..len {
				unsafe { writer[i].as_mut_ptr().drop_in_place() };
			}
		}
	}
}

struct ReadHandle<'a, T> {
	valid_prefix: usize,
	reader: MutexReader<'a, Vec<MaybeUninit<SyncUnsafeCell<T>>>>,
}

impl<T> Deref for ReadHandle<'_, T> {
	type Target = [T];

	fn deref(&self) -> &Self::Target {
		unsafe { mem::transmute(&self.reader[0..self.valid_prefix]) }
	}
}

struct SyncUnsafeCell<T>(UnsafeCell<T>);

unsafe impl<T: Send> Send for SyncUnsafeCell<T> {}
unsafe impl<T: Sync> Sync for SyncUnsafeCell<T> {}

#[cfg(test)]
mod tests {
	use std::{sync::Arc, thread};

	use super::ConcurrentVec;

	#[test]
	fn basic_push() {
		const N_THREADS: usize = 10;
		const PER_THREAD: usize = 10;
		let v = Arc::new(ConcurrentVec::<usize>::with_capacity(0));
		let threads = (0..N_THREADS).map(|i| {
			let v = v.clone();
			thread::spawn(move || {
				let mut got = Vec::new();
				for j in 0..PER_THREAD {
					got.push(v.push(i * PER_THREAD + j));
				}

				got
			})
		});

		let mut results = threads
			.into_iter()
			.flat_map(|x| x.join().unwrap())
			.collect::<Vec<usize>>();

		results.sort();
		assert_eq!(results.len(), N_THREADS * PER_THREAD);
		assert_eq!(
			results,
			(0..(N_THREADS * PER_THREAD)).collect::<Vec<usize>>()
		);
		let slice = v.read();
		assert_eq!(slice.len(), N_THREADS * PER_THREAD);
		let mut sorted = slice.to_vec();
		sorted.sort();
		assert_eq!(
			sorted,
			(0..(N_THREADS * PER_THREAD)).collect::<Vec<usize>>()
		);
	}
}
