#![allow(clippy::wildcard_imports)]

use std::iter::Peekable;

use anyhow::Result;
use egg::*;

pub type EGraph<L = Operations> = egg::EGraph<L, ()>;
pub type Rewrite<L = Operations> = egg::Rewrite<L, ()>;

const BF: &str = include_str!("../../../../programs/test.bf");

define_language! {
	pub enum Operations {
		ConstantCell(i8),
		ConstantShift(i32),
		"add" = Inc(Id),
		"shift" = ShiftPtr(Id),
		"output" = Output,
		"input" = Input,
		"seq" = Seq([Id; 2]),
		"loop" = Loop(Id),
		"clear" = Clear,
	}
}

fn main() -> Result<()> {
	let expr = parse(BF);

	let mut runner = Runner::default()
		.with_explanations_enabled()
		.with_explanation_length_optimization()
		.with_expr(&expr)
		.run(&make_rules());

	let end = {
		let extractor = Extractor::new(&runner.egraph, AstSize);

		let (_, end) = extractor.find_best(runner.roots[0]);

		end
	};

	let explanation = runner.explain_equivalence(&expr, &end).get_flat_string();

	println!("{end}\n{explanation}\n");

	Ok(())
}

fn make_rules() -> Vec<Rewrite> {
	vec![rewrite!("clear-cell"; "(loop (add -1))" => "clear")]
}

fn parse(s: &str) -> RecExpr<Operations> {
	let mut expr = RecExpr::default();
	let mut chars = s.chars().peekable();

	parse_inner(&mut chars, &mut expr);
	expr
}

fn build_seq(expr: &mut RecExpr<Operations>, ids: Vec<Id>) -> Id {
	let mut ids = ids.into_iter().rev();
	let mut current = ids.next().expect("empty sequence");

	for id in ids {
		current = expr.add(Operations::Seq([id, current]));
	}

	current
}

fn parse_inner<I>(chars: &mut Peekable<I>, expr: &mut RecExpr<Operations>) -> Id
where
	I: Iterator<Item = char>,
{
	let mut instructions = Vec::new();

	while let Some(&c) = chars.peek() {
		match c {
			'+' | '-' => {
				let mut count = 0i8;
				while let Some(&c2) = chars.peek() {
					match c2 {
						'+' => {
							count = count.wrapping_add(1);
							chars.next();
						}
						'-' => {
							count = count.wrapping_sub(1);
							chars.next();
						}
						_ => break,
					}
				}

				if !matches!(count, 0) {
					let const_id = expr.add(Operations::ConstantCell(count));
					let add_id = expr.add(Operations::Inc(const_id));
					instructions.push(add_id);
				}
			}
			'>' | '<' => {
				let mut shifted = 0i32;
				while let Some(&c2) = chars.peek() {
					match c2 {
						'>' => {
							shifted = shifted.wrapping_add(1);
							chars.next();
						}
						'<' => {
							shifted = shifted.wrapping_sub(1);
							chars.next();
						}
						_ => break,
					}

					if !matches!(shifted, 0) {
						let const_id = expr.add(Operations::ConstantShift(shifted));
						let shift_id = expr.add(Operations::ShiftPtr(const_id));

						instructions.push(shift_id);
					}
				}
			}
			'.' => {
				chars.next();
				instructions.push(expr.add(Operations::Output));
			}
			',' => {
				chars.next();
				instructions.push(expr.add(Operations::Input));
			}
			'[' => {
				chars.next();
				let loop_body = parse_inner(chars, expr);
				instructions.push(expr.add(Operations::Loop(loop_body)));
			}
			']' => {
				chars.next();
				break;
			}
			_ => {
				chars.next();
			}
		}
	}

	build_seq(expr, instructions)
}
