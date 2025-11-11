use frick_operations::parse;

fn main() {
	let src = std::env::args().nth(1).unwrap();

	let parsed = parse(src).unwrap();

	for op in parsed {
		println!("{op:?}");
	}
}
