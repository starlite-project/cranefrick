use std::hint::black_box;

use cranefrick_ast::{BrainAst, Logos};
use cranefrick_ir::AstParser;
use criterion::{Criterion, criterion_group, criterion_main};

const BASIC: &str = include_str!("../../../programs/hello_world.bf");

fn setup(source: &str) -> AstParser {
	let source_iter = BrainAst::lexer(source)
		.spanned()
		.filter_map(|(tok, span)| Some((tok.ok()?, span.into())));

	AstParser::new(source_iter, source.to_owned())
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
