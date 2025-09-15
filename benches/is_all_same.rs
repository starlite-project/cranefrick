use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use rand::Rng as _;

const ARRAY_SIZE: usize = 171;

fn setup_different() -> [u8; ARRAY_SIZE] {
	let mut array = [0; ARRAY_SIZE];
	let mut rng = rand::rng();

	rng.fill(&mut array);

	array
}

fn setup_same() -> [u8; ARRAY_SIZE] {
	let mut rng = rand::rng();

	let value = rng.random();

	[value; ARRAY_SIZE]
}

fn is_all_same1(arr: &[u8]) -> bool {
	if arr.is_empty() {
		return true;
	}

	let first = arr[0];
	arr.iter().all(|&item| item == first)
}

fn is_all_same2(arr: &[u8]) -> bool {
	arr.iter().min() == arr.iter().max()
}

fn is_all_same3(arr: &[u8]) -> bool {
	arr.windows(2).all(|w| w[0] == w[1])
}

fn is_all_same4(arr: &[u8]) -> bool {
	for c in arr.windows(2) {
		if c[0] != c[1] {
			return false;
		}
	}

	true
}

fn is_all_same5(arr: &[u8]) -> bool {
	match arr {
		[] | [_] => true,
		[head, second, ..] if head != second => false,
		[_, rest @ ..] => is_all_same5(rest),
	}
}

fn is_all_same6(arr: &[u8]) -> bool {
	let mut iter = arr.iter();

	let first = iter.next();

	iter.fold(first, |acc, item| {
		acc.and_then(|stored| if stored == item { Some(stored) } else { None })
	})
	.is_some()
}

fn is_all_same7(arr: &[u8]) -> bool {
	match arr.split_first() {
		Some((first, remaining)) => remaining.iter().all(|item| *item == *first),
		None => true,
	}
}

fn is_all_same8(arr: &[u8]) -> bool {
	arr.iter()
		.fold((true, None), |acc, elem| {
			if acc.1.is_some() {
				(acc.0 && (acc.1.unwrap() == elem), Some(elem))
			} else {
				(true, Some(elem))
			}
		})
		.0
}

fn is_all_same9(arr: &[u8]) -> bool {
	arr.iter()
		.fold((true, None), |acc, elem| {
			if let Some(prev) = acc.1 {
				(acc.0 && (prev == elem), Some(elem))
			} else {
				(true, Some(elem))
			}
		})
		.0
}

fn is_all_same10(arr: &[u8]) -> bool {
	arr.first()
		.map(|first| arr.iter().all(|x| x == first))
		.unwrap_or(true)
}

fn is_all_same_different(c: &mut Criterion) {
	let mut g = c.benchmark_group("is_all_same_different");

	g.bench_function("is_all_same1", |b| {
		b.iter_batched(
			setup_different,
			|data| assert!(!is_all_same1(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same2", |b| {
		b.iter_batched(
			setup_different,
			|data| assert!(!is_all_same2(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same3", |b| {
		b.iter_batched(
			setup_different,
			|data| assert!(!is_all_same3(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same4", |b| {
		b.iter_batched(
			setup_different,
			|data| assert!(!is_all_same4(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same5", |b| {
		b.iter_batched(
			setup_different,
			|data| assert!(!is_all_same5(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same6", |b| {
		b.iter_batched(
			setup_different,
			|data| assert!(!is_all_same6(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same7", |b| {
		b.iter_batched(
			setup_different,
			|data| assert!(!is_all_same7(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same8", |b| {
		b.iter_batched(
			setup_different,
			|data| assert!(!is_all_same8(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same9", |b| {
		b.iter_batched(
			setup_different,
			|data| assert!(!is_all_same9(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same10", |b| {
		b.iter_batched(
			setup_different,
			|data| assert!(!is_all_same10(&data)),
			BatchSize::SmallInput,
		);
	});
}

fn is_all_same_same(c: &mut Criterion) {
	let mut g = c.benchmark_group("is_all_same_same");

	g.bench_function("is_all_same1", |b| {
		b.iter_batched(
			setup_same,
			|data| assert!(is_all_same1(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same2", |b| {
		b.iter_batched(
			setup_same,
			|data| assert!(is_all_same2(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same3", |b| {
		b.iter_batched(
			setup_same,
			|data| assert!(is_all_same3(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same4", |b| {
		b.iter_batched(
			setup_same,
			|data| assert!(is_all_same4(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same5", |b| {
		b.iter_batched(
			setup_same,
			|data| assert!(is_all_same5(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same6", |b| {
		b.iter_batched(
			setup_same,
			|data| assert!(is_all_same6(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same7", |b| {
		b.iter_batched(
			setup_same,
			|data| assert!(is_all_same7(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same8", |b| {
		b.iter_batched(
			setup_same,
			|data| assert!(is_all_same8(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same9", |b| {
		b.iter_batched(
			setup_same,
			|data| assert!(is_all_same9(&data)),
			BatchSize::SmallInput,
		);
	});

	g.bench_function("is_all_same10", |b| {
		b.iter_batched(
			setup_same,
			|data| assert!(is_all_same10(&data)),
			BatchSize::SmallInput,
		);
	});
}

criterion_group!(benches, is_all_same_different, is_all_same_same);
criterion_main!(benches);
