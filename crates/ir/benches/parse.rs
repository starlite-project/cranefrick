use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use frick_ir::AstParser;

const HELLO_WORLD: &str = include_str!("../../../programs/hello_world.bf");

const HELLO_WORLD_TEST: &str = include_str!("../../../programs/tests/hello_world_test.bf");

const MANDLEBROT: &str = include_str!("../../../programs/mandlebrot.bf");

const AWIB: &str = include_str!("../../../programs/awib.bf");

fn setup(source: &str) -> AstParser {
	AstParser::new(
		source
			.chars()
			.filter(|c| matches!(c, '[' | ']' | '>' | '<' | '+' | '-' | '.' | ','))
			.collect(),
	)
}

fn bench_basic(c: &mut Criterion) {
	c.bench_function("hello_world parse", |b| {
		b.iter(|| {
			assert!(setup(black_box(HELLO_WORLD)).parse().is_ok());
		});
	});

	c.bench_function("hello_world_test parse", |b| {
		b.iter(|| {
			assert!(setup(black_box(HELLO_WORLD_TEST)).parse().is_ok());
		});
	});

	c.bench_function("mandlebrot parse", |b| {
		b.iter(|| {
			assert!(setup(black_box(MANDLEBROT)).parse().is_ok());
		});
	});

	c.bench_function("awib parse", |b| {
		b.iter(|| {
			assert!(setup(black_box(AWIB)).parse().is_ok());
		});
	});
}

criterion_group!(benches, bench_basic);

criterion_main!(benches);
