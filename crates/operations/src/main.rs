use frick_operations::parse;

fn main() {
	// let src = std::fs::read_to_string(std::env::args().nth(1).unwrap())
	// 	.unwrap()
	// 	.chars()
	// 	.collect::<String>();
	let src = std::env::args().nth(1).unwrap();
	println!("{:?}", parse(src).unwrap());
}
