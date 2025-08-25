use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
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
	let mut group = c.benchmark_group("parse");
	for (name, value) in [
		("hello_world", HELLO_WORLD),
		("hello_world_test", HELLO_WORLD_TEST),
		("mandlebrot", MANDLEBROT),
		("awib", AWIB),
	] {
		group.throughput(Throughput::Bytes(value.len() as u64));
		group.bench_with_input(
			BenchmarkId::new(format!("{name} parse"), value.len()),
			&value,
			|b, &i| {
				b.iter(|| assert!(setup(i).parse().is_ok()));
			},
		);
	}
}

criterion_group!(benches, bench_basic);

criterion_main!(benches);
