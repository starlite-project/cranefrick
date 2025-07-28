#![allow(clippy::wildcard_imports)]

use std::iter::Peekable;

use anyhow::Result;
use egg::*;

pub type EGraph<L = Operation> = egg::EGraph<L, ()>;
pub type Rewrite<L = Operation> = egg::Rewrite<L, ()>;

const BF: &str = include_str!("../../../programs/test.bf");

define_language! {
	pub enum Operation {
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

fn parse(s: &str) -> RecExpr<Operation> {
	let mut expr = RecExpr::default();
	let mut chars = s.chars().peekable();

	parse_inner(&mut chars, &mut expr);
	expr
}

fn build_seq(expr: &mut RecExpr<Operation>, ids: Vec<Id>) -> Id {
	let mut ids = ids.into_iter().rev();
	let mut current = ids.next().expect("empty sequence");

	for id in ids {
		current = expr.add(Operation::Seq([id, current]));
	}

	current
}

fn parse_inner<I>(chars: &mut Peekable<I>, expr: &mut RecExpr<Operation>) -> Id
where
	I: Iterator<Item = char>,
{
	let mut instructions = Vec::new();

	while let Some(&c) = chars.peek() {
		match c {
			'+' => {
				chars.next();
				let const_id = expr.add(Operation::ConstantCell(1));
				let add_id = expr.add(Operation::Inc(const_id));
				instructions.push(add_id);
			}
			'-' => {
				chars.next();
				let const_id = expr.add(Operation::ConstantCell(-1));
				let add_id = expr.add(Operation::Inc(const_id));
				instructions.push(add_id);
			}
			'>' => {
				chars.next();
				let const_id = expr.add(Operation::ConstantShift(1));
				let shift_id = expr.add(Operation::ShiftPtr(const_id));
				instructions.push(shift_id);
			}
			'<' => {
				chars.next();
				let const_id = expr.add(Operation::ConstantShift(-1));
				let shift_id = expr.add(Operation::ShiftPtr(const_id));
				instructions.push(shift_id);
			}
			'.' => {
				chars.next();
				instructions.push(expr.add(Operation::Output));
			}
			',' => {
				chars.next();
				instructions.push(expr.add(Operation::Input));
			}
			'[' => {
				chars.next();
				let loop_body = parse_inner(chars, expr);
				instructions.push(expr.add(Operation::Loop(loop_body)));
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
