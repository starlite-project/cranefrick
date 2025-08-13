#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod bitset;
mod concurrent_vec;
mod notif;
mod par_writer;
mod resettable_oncelock;

use std::{
	cell::UnsafeCell,
	mem,
	ops::{Deref, DerefMut},
	ptr,
	sync::{
		Arc,
		atomic::{Ordering, fence},
	},
};

use arc_swap::{ArcSwap, Guard};

pub use self::{bitset::*, concurrent_vec::*, notif::*, par_writer::*, resettable_oncelock::*};

pub struct ReadOptimizedLock<T> {
	token: ArcSwap<ReadToken>,
	data: UnsafeCell<T>,
}

impl<T> ReadOptimizedLock<T> {
	pub fn new(data: T) -> Self {
		Self {
			token: ArcSwap::from_pointee(ReadToken::ReadOk(TriggerWhenDone::default())),
			data: UnsafeCell::new(data),
		}
	}

	pub fn into_inner(self) -> T {
		self.data.into_inner()
	}

	pub const fn as_mut_ref(&mut self) -> &mut T {
		self.data.get_mut()
	}

	pub fn read(&self) -> MutexReader<'_, T> {
		loop {
			let guard = self.token.load();
			match guard.as_ref() {
				ReadToken::ReadOk(..) => {
					fence(Ordering::Acquire);
					break MutexReader {
						data: unsafe { &*self.data.get() },
						guard,
					};
				}
				ReadToken::WriteOngoing(n) => {
					let n = n.clone();
					mem::drop(guard);
					n.wait();
				}
			}
		}
	}

	pub fn lock(&self) -> MutexWriter<'_, T> {
		loop {
			let guard = self.token.load();
			match guard.as_ref() {
				ReadToken::ReadOk(n) => {
					let unblock_waiters = Arc::new(Notification::default());
					let write_token = ReadToken::WriteOngoing(unblock_waiters.clone());
					let readers_done = n.0.clone();
					let prev = self.token.compare_and_swap(&guard, Arc::new(write_token));
					if !ptr::eq(prev.as_ref(), guard.as_ref()) {
						continue;
					}

					mem::drop((guard, prev));
					self.token.rcu(Clone::clone);

					readers_done.wait();

					break MutexWriter {
						lock: self,
						unblock: unblock_waiters,
					};
				}
				ReadToken::WriteOngoing(n) => {
					let n = n.clone();
					mem::drop(guard);
					n.wait();
				}
			}
		}
	}
}

unsafe impl<T: Send> Send for ReadOptimizedLock<T> {}
unsafe impl<T: Sync> Sync for ReadOptimizedLock<T> {}

pub struct MutexReader<'lock, T> {
	data: &'lock T,
	#[allow(dead_code, reason = "drop guard")]
	guard: Guard<Arc<ReadToken>>,
}

impl<T> Deref for MutexReader<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.data
	}
}

pub struct MutexWriter<'lock, T> {
	lock: &'lock ReadOptimizedLock<T>,
	unblock: Arc<Notification>,
}

impl<T> Deref for MutexWriter<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.lock.data.get() }
	}
}

impl<T> DerefMut for MutexWriter<'_, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.lock.data.get() }
	}
}

impl<T> Drop for MutexWriter<'_, T> {
	fn drop(&mut self) {
		self.lock
			.token
			.store(Arc::new(ReadToken::ReadOk(TriggerWhenDone::default())));

		self.unblock.notify();
	}
}

#[derive(Default)]
struct TriggerWhenDone(Arc<Notification>);

impl Drop for TriggerWhenDone {
	fn drop(&mut self) {
		self.0.notify();
	}
}

enum ReadToken {
	ReadOk(TriggerWhenDone),
	WriteOngoing(Arc<Notification>),
}

#[cfg(test)]
mod tests {
	use std::{
		mem,
		sync::Arc,
		thread::{self, sleep},
		time::Duration,
	};

	use super::{Notification, ReadOptimizedLock};

	#[test]
	fn simple_mutex() {
		for _ in 0..50 {
			let m = Arc::new(ReadOptimizedLock::new(0));
			let read_guard_1 = m.read();
			let read_guard_2 = m.read();
			assert_eq!(*read_guard_1, 0);
			assert_eq!(*read_guard_2, 0);
			let locked = Arc::new(Notification::new());
			let locked_inner = locked.clone();
			let m_inner = m.clone();
			let writer = thread::spawn(move || {
				let mut lock = m_inner.lock();
				locked_inner.notify();
				*lock = 5;
			});

			sleep(Duration::from_millis(1));
			assert!(!locked.has_been_notified());
			mem::drop(read_guard_1);
			sleep(Duration::from_millis(1));
			assert!(!locked.has_been_notified());
			mem::drop(read_guard_2);
			locked.wait();
			writer.join().unwrap();
			assert_eq!(*m.read(), 5);
		}
	}
}
