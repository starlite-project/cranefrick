use std::ptr;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

const ARRAY_SIZE: usize = 1027;

fn setup_array() -> &'static [u8] {
	static ARRAY: [u8; ARRAY_SIZE] = [0; ARRAY_SIZE];

	&ARRAY
}

fn is_all_zero_iter_all(x: &[u8]) -> bool {
	x.iter().all(|x| matches!(x, 0))
}

fn is_all_zero_for(x: &[u8]) -> bool {
	for &x in x {
		if !matches!(x, 0) {
			return false;
		}
	}

	true
}

fn is_all_zero_ptrs(x: &[u8]) -> bool {
	unsafe {
		let mut p = x.as_ptr();
		let end = p.add(x.len());
		while p < end {
			if ptr::read(p) != 0 {
				return false;
			}

			p = p.add(1);
		}
	}

	true
}

fn is_all_zero_align_up(x: &[u8]) -> bool {
	let (prefix, aligned, suffix) = unsafe { x.align_to::<u128>() };

	let c = |&x| matches!(x, 0);
	let c2 = |&x| matches!(x, 0);

	prefix.iter().all(c) && suffix.iter().all(c) && aligned.iter().all(c2)
}

fn is_all_zero_bench(c: &mut Criterion) {
	let mut g = c.benchmark_group("is_all_zero");

	g.bench_function("is_all_zero_iter_all", |b| {
		b.iter_batched(
			setup_array,
			|data| assert!(is_all_zero_iter_all(data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_zero_for", |b| {
		b.iter_batched(
			setup_array,
			|data| assert!(is_all_zero_for(data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_zero_ptrs", |b| {
		b.iter_batched(
			setup_array,
			|data| assert!(is_all_zero_ptrs(data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_zero_align_up", |b| {
		b.iter_batched(
			setup_array,
			|data| assert!(is_all_zero_align_up(data)),
			BatchSize::SmallInput,
		);
	});

	g.finish();
}

criterion_group!(benches, is_all_zero_bench);
criterion_main!(benches);
