use std::{
	cmp, mem,
	ops::{Deref, Range},
	ptr,
	sync::atomic::{AtomicUsize, Ordering},
};

use super::{MutexReader, ReadOptimizedLock};

pub struct ParallelVecWriter<T> {
	data: ReadOptimizedLock<Vec<T>>,
	end_len: AtomicUsize,
}

impl<T> ParallelVecWriter<T> {
	#[must_use]
	pub fn new(data: Vec<T>) -> Self {
		let start = data.len();
		let end_len = AtomicUsize::new(start);

		Self {
			data: ReadOptimizedLock::new(data),
			end_len,
		}
	}

	pub fn read_access(&self) -> impl Deref<Target = [T]> + '_ {
		struct PrefixReader<'a, T> {
			reader: MutexReader<'a, Vec<T>>,
		}

		impl<T> Deref for PrefixReader<'_, T> {
			type Target = [T];

			fn deref(&self) -> &Self::Target {
				self.reader.as_slice()
			}
		}

		PrefixReader {
			reader: self.data.read(),
		}
	}

	pub fn unsafe_read_access(&self) -> UnsafeReadAccess<'_, T> {
		UnsafeReadAccess {
			reader: self.data.read(),
		}
	}

	pub fn with_index<R>(&self, idx: usize, f: impl FnOnce(&T) -> R) -> R {
		f(&self.read_access()[idx])
	}

	pub fn with_slice<R>(&self, slice: Range<usize>, f: impl FnOnce(&[T]) -> R) -> R {
		f(&self.read_access()[slice])
	}

	pub fn write_contents(&self, items: impl ExactSizeIterator<Item = T>) -> usize {
		let start = self.end_len.fetch_add(items.len(), Ordering::AcqRel);
		let end = start + items.len();
		let reader = self.data.read();
		let current_len = reader.len();
		let current_cap = reader.capacity();
		mem::drop(reader);

		if current_cap < end {
			let mut writer = self.data.lock();
			if writer.capacity() < end {
				let new_cap = cmp::max(end, current_cap * 2);
				writer.reserve(new_cap - current_len);
			}
		}

		unsafe {
			self.write_contents_at(items, start);
		}
		start
	}

	pub fn finish(self) -> Vec<T> {
		let mut res = self.data.into_inner();

		unsafe {
			res.set_len(self.end_len.load(Ordering::Acquire));
		}

		res
	}

	pub fn take(&mut self) -> Vec<T> {
		let mut res = mem::take(self.data.as_mut_ref());

		unsafe {
			res.set_len(self.end_len.load(Ordering::Acquire));
		}

		self.end_len.store(0, Ordering::Release);
		res
	}

	unsafe fn write_contents_at(&self, items: impl ExactSizeIterator<Item = T>, start: usize) {
		let mut written = 0;
		let expected = items.len();
		let reader = self.data.read();
		debug_assert!(reader.capacity() >= start + items.len());
		let mut mut_ptr = unsafe { reader.as_ptr().cast_mut().add(start) };
		for item in items {
			written += 1;
			unsafe { ptr::write(mut_ptr, item) };
			mut_ptr = unsafe { mut_ptr.offset(1) };
		}

		assert_eq!(
			written, expected,
			"passed ExactSizeIterator with incorrect number of items"
		);
	}
}

pub struct UnsafeReadAccess<'a, T> {
	reader: MutexReader<'a, Vec<T>>,
}

impl<T> UnsafeReadAccess<'_, T> {
	#[must_use]
	pub unsafe fn get_unchecked(&self, idx: usize) -> &T {
		unsafe { &*self.reader.as_ptr().add(idx) }
	}

	#[must_use]
	pub unsafe fn get_slice_unchecked(&self, slice: Range<usize>) -> &[T] {
		let start = unsafe { self.reader.as_ptr().add(slice.start) };
		unsafe { std::slice::from_raw_parts(start, slice.end - slice.start) }
	}
}

#[cfg(test)]
mod tests {
	use std::{
		sync::Arc,
		thread::{self, sleep},
		time::Duration,
	};

	use super::ParallelVecWriter;
	use crate::Notification;

	#[test]
	fn basic_write() {
		const N_THREADS: usize = 10;
		const PER_THREAD: usize = 10;
		let finish = Arc::new(Notification::new());
		let v = (0..100).collect::<Vec<usize>>();
		let v = Arc::new(ParallelVecWriter::new(v));
		let threads = (0..N_THREADS)
			.map(|i| {
				let finish = finish.clone();
				let v = v.clone();
				thread::spawn(move || {
					let dst = v.write_contents((0..PER_THREAD).map(|j| i * PER_THREAD + j + 100));
					assert!(dst.is_multiple_of(10));
					finish.wait();
				})
			})
			.collect::<Vec<_>>();

		sleep(Duration::from_millis(100));
		for i in 0..100 {
			v.with_index(i, |x| assert_eq!(*x, i));
		}

		v.with_slice(0..100, |x| assert_eq!(x, (0..100).collect::<Vec<_>>()));
		finish.notify();
		threads.into_iter().for_each(|x| x.join().unwrap());
		let mut v = Arc::try_unwrap(v).ok().unwrap().finish();
		v.sort();
		assert_eq!(v, (0..200).collect::<Vec<usize>>());
	}
}
