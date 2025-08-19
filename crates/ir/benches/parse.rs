use std::hint::black_box;

use cranefrick_ir::AstParser;
use criterion::{Criterion, criterion_group, criterion_main};

const BASIC: &str = include_str!("../../../programs/hello_world.bf");

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
			assert!(setup(black_box(BASIC)).parse().is_ok());
		});
	});
}

criterion_group!(benches, bench_basic);

criterion_main!(benches);
