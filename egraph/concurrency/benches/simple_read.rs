use std::{ops::Deref, sync::RwLock};

use cranefrick_egraph_concurrency::{MutexReader, ReadOptimizedLock};
use divan::{Bencher, counter::ItemsCount};

fn main() {
	divan::main();
}

trait ReadLock<T>: Send + Sync {
	type Guard<'a>: Deref<Target = T>
	where
		Self: 'a;

	fn new(data: T) -> Self;

	fn read(&self) -> Self::Guard<'_>;
}

impl<T> ReadLock<T> for RwLock<T>
where
	T: Send + Sync,
{
	type Guard<'a>
		= std::sync::RwLockReadGuard<'a, T>
	where
		Self: 'a;

	fn new(data: T) -> Self {
		Self::new(data)
	}

	fn read(&self) -> Self::Guard<'_> {
		self.read().unwrap()
	}
}

impl<T> ReadLock<T> for ReadOptimizedLock<T>
where
	T: Send + Sync,
{
	type Guard<'a>
		= MutexReader<'a, T>
	where
		Self: 'a;

	fn new(data: T) -> Self {
		Self::new(data)
	}

	fn read(&self) -> Self::Guard<'_> {
		self.read()
	}
}

#[divan::bench(threads = [1, 2, 4, 8, 16, 20], types = [ReadOptimizedLock<usize>, RwLock<usize>], sample_size = 100)]
fn read_contention<T: ReadLock<usize>>(bencher: Bencher<'_, '_>) {
	let lock = T::new(0);
	bencher.bench(|| {
		divan::black_box(*lock.read());
	});
}

#[divan::bench(types = [ReadOptimizedLock<usize>, RwLock<usize>], consts = [1, 2, 4, 8, 16, 20], sample_count = 50)]
fn read_throughput<T: ReadLock<usize>, const N: usize>(bencher: Bencher<'_, '_>) {
	const TOTAL_ITEMS: usize = 1_000_000;
	const BATCH_SIZE: usize = 1_000;
	const N_BATCHES: usize = TOTAL_ITEMS / BATCH_SIZE;

	let pool = rayon::ThreadPoolBuilder::new()
		.num_threads(N)
		.build()
		.unwrap();

	bencher
		.with_inputs(|| ())
		.input_counter(|()| ItemsCount::new(TOTAL_ITEMS))
		.bench_values(|()| {
			let lock = T::new(0);
			pool.scope(|scope| {
				for _ in 0..N_BATCHES {
					scope.spawn(|_| {
						for _ in 0..BATCH_SIZE {
							divan::black_box(*lock.read());
						}
					});
				}
			});
		});
}
