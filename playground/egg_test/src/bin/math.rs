#![allow(clippy::wildcard_imports)]

use color_eyre::Result;
use egg::*;
use egg_test::run_expr;
use ordered_float::NotNan;
use rayon::prelude::*;

pub type Constant = NotNan<f64>;

pub type EGraph = egg::EGraph<Math, ConstantFold>;
pub type Rewrite = egg::Rewrite<Math, ConstantFold>;

define_language! {
	pub enum Math {
		"d" = Diff([Id; 2]),
		"i" = Integral([Id; 2]),

		"+" = Add([Id; 2]),
		"-" = Sub([Id; 2]),
		"*" = Mul([Id; 2]),
		"/" = Div([Id; 2]),
		"pow" = Pow([Id; 2]),
		"ln" = Ln(Id),
		"sqrt" = Sqrt(Id),

		"sin" = Sin(Id),
		"cos" = Cos(Id),

		Constant(Constant),
		Symbol(Symbol),
	}
}

pub struct MathCostFn;

impl CostFunction<Math> for MathCostFn {
	type Cost = usize;

	fn cost<C>(&mut self, enode: &Math, mut costs: C) -> Self::Cost
	where
		C: FnMut(Id) -> Self::Cost,
	{
		let op_cost = match enode {
			Math::Diff(..) | Math::Integral(..) => 100,
			_ => 1,
		};

		enode.fold(op_cost, |sum, i| sum + costs(i))
	}
}

#[derive(Default)]
pub struct ConstantFold;

impl Analysis<Math> for ConstantFold {
	type Data = Option<(Constant, PatternAst<Math>)>;

	fn make(egraph: &mut egg::EGraph<Math, Self>, enode: &Math) -> Self::Data {
		let x = |i: &Id| egraph[*i].data.as_ref().map(|d| d.0);

		Some(match enode {
			Math::Constant(c) => (*c, c.to_string().parse().unwrap()),
			Math::Add([a, b]) => (
				x(a)? + x(b)?,
				format!("(+ {} {})", x(a)?, x(b)?).parse().unwrap(),
			),
			Math::Sub([a, b]) => (
				x(a)? - x(b)?,
				format!("(- {} {})", x(a)?, x(b)?).parse().unwrap(),
			),
			Math::Mul([a, b]) => (
				x(a)? * x(b)?,
				format!("(* {} {})", x(a)?, x(b)?).parse().unwrap(),
			),
			Math::Div([a, b]) if x(b) != Some(NotNan::new(0.0).unwrap()) => (
				x(a)? / x(b)?,
				format!("(/ {} {})", x(a)?, x(b)?).parse().unwrap(),
			),
			_ => return None,
		})
	}

	fn merge(&mut self, a: &mut Self::Data, b: Self::Data) -> DidMerge {
		merge_option(a, b, |a, b| {
			assert_eq!(a.0, b.0, "merged non-equal constants");
			DidMerge(false, false)
		})
	}

	fn modify(egraph: &mut EGraph, id: Id) {
		let data = egraph[id].data.clone();
		if let Some((c, pat)) = data {
			if egraph.are_explanations_enabled() {
				egraph.union_instantiations(
					&pat,
					&c.to_string().parse().unwrap(),
					&Subst::default(),
					"constant_fold".to_owned(),
				);
			} else {
				let added = egraph.add(Math::Constant(c));
				egraph.union(id, added);
			}

			egraph[id].nodes.retain(Language::is_leaf);

			#[cfg(debug_assertions)]
			egraph[id].assert_unique_leaves();
		}
	}
}

fn main() -> Result<()> {
	color_eyre::install()?;

	let exprs = &[
		"(i (ln x) x)",
		"(i (+ x (cos x)) x)",
		"(i (* (cos x) x) x)",
		"(d x (+ 1 (* 2 x)))",
		"(d x (- (pow x 3) (* 7 (pow x 2))))",
		"(+ (* y (+ x y)) (- (+ x 2) (+ x x)))",
		"(/ 1 (- (/ (+ 1 (sqrt five)) 2) (/ (- 1 (sqrt five)) 2)))",
		"(+ ?a (+ ?b ?c))",
		"(+ (+ ?a ?b) ?c)",
		"(* ?a (* ?b ?c))",
		"(* (* ?a ?b) ?c)",
		"(+ ?a (* -1 ?b))",
		"(* ?a (pow ?b -1))",
		"(* ?a (+ ?b ?c))",
		"(pow ?a (+ ?b ?c))",
		"(+ (* ?a ?b) (* ?a ?c))",
		"(* (pow ?a ?b) (pow ?a ?c))",
		"(* ?x (/ 1 ?x))",
		"(d ?x (+ ?a ?b))",
		"(+ (d ?x ?a) (d ?x ?b))",
		"(d ?x (* ?a ?b))",
		"(+ (* ?a (d ?x ?b)) (* ?b (d ?x ?a)))",
		"(d ?x (sin ?x))",
		"(d ?x (cos ?x))",
		"(* -1 (sin ?x))",
		"(* -1 (cos ?x))",
		"(i (cos ?x) ?x)",
		"(i (sin ?x) ?x)",
		"(d ?x (ln ?x))",
		"(d ?x (pow ?f ?g))",
		"(* (pow ?f ?g) (+ (* (d ?x ?f) (/ ?g ?f)) (* (d ?x ?g) (ln ?f))))",
		"(i (pow ?x ?c) ?x)",
		"(/ (pow ?x (+ ?c 1)) (+ ?c 1))",
		"(i (+ ?f ?g) ?x)",
		"(i (- ?f ?g) ?x)",
		"(+ (i ?f ?x) (i ?g ?x))",
		"(- (i ?f ?x) (i ?g ?x))",
		"(i (* ?a ?b) ?x)",
		"(- (* ?a (i ?b ?x)) (i (* (d ?x ?a) (i ?b ?x)) ?x))",
		"(+ 1 (+ 2 (+ 3 (+ 4 (+ 5 (+ 6 7))))))",
		"(+ x (+ x (+ x x)))",
		"(* (pow 2 x) (pow 2 y))",
		"(+ 1 (- a (* (- 2 1) a)))",
	];

	let results = exprs
		.par_iter()
		.map(|expr| run_expr(expr, &rules(), AstSize))
		.collect::<Result<Vec<_>>>()?;

	for (end, expl) in results {
		println!("{end}\n{expl}\n");
	}

	Ok(())
}

fn rules() -> Vec<Rewrite> {
	let mut rules = vec![
		rewrite!("comm-add"; "(+ ?a ?b)" => "(+ ?b ?a)"),
		rewrite!("comm-mul"; "(* ?a ?b)" => "(* ?b ?a)"),
		rewrite!("assoc-add"; "(+ ?a (+ ?b ?c))" => "(+ (+ ?a ?b) ?c)"),
		rewrite!("assoc-mul"; "(* ?a (* ?b ?c))" => "(* (* ?a ?b) ?c)"),
		rewrite!("sub-canon"; "(- ?a ?b)" => "(+ ?a (* -1 ?b))"),
		rewrite!("div-canon"; "(/ ?a ?b)" => "(* ?a (pow ?b -1))" if is_not_zero("?b")),
		rewrite!("zero-mul"; "(* ?a 0)" => "0"),
		rewrite!("cancel-sub"; "(- ?a ?a)" => "0"),
		rewrite!("cancel-div"; "(/ ?a ?a)" => "1" if is_not_zero("?a")),
		rewrite!("pow-mul"; "(* (pow ?a ?b) (pow ?a ?c))" => "(pow ?a (+ ?b ?c))"),
		rewrite!("pow0"; "(pow ?x 0)" => "1" if is_not_zero("?x")),
		rewrite!("pow1"; "(pow ?x 1)" => "?x"),
		rewrite!("pow-recip"; "(pow ?x -1)" => "(/ 1 ?x)" if is_not_zero("?x")),
		rewrite!("recip-mul-div"; "(* ?x (/ 1 ?x))" => "1" if is_not_zero("?x")),
		rewrite!("d-variable"; "(d ?x ?x)" => "1" if is_sym("?x")),
		rewrite!("d-constant"; "(d ?x ?c)" => "0" if is_sym("?x") if is_const_or_distinct_var("?c", "?x")),
		rewrite!("d-add"; "(d ?x (+ ?a ?b))" => "(+ (d ?x ?a) (d ?x ?b))"),
		rewrite!("d-mul"; "(d ?x (* ?a ?b))" => "(+ (* ?a (d ?x ?b)) (* ?b (d ?x ?a)))"),
		rewrite!("d-sin"; "(d ?x (sin ?x))" => "(cos ?x)"),
		rewrite!("d-cos"; "(d ?x (cos ?x))" => "(* -1 (sin ?x))"),
		rewrite!("d-ln"; "(d ?x (ln ?x))" => "(/ 1 ?x)" if is_not_zero("?x")),
		rewrite!("d-power";
			"(d ?x (pow ?f ?g))" =>
			"(* (pow ?f ?g)
            (+ (* (d ?x ?f)
                  (/ ?g ?f))
               (* (d ?x ?g)
                  (ln ?f))))"
			if is_not_zero("?f")
			if is_not_zero("?g")
		),
		rewrite!("i-one"; "(i 1 ?x)" => "?x"),
		rewrite!("i-power-const"; "(i (pow ?x ?c) ?x)" => "(/ (pow ?x (+ ?c 1)) (+ ?c 1))" if is_const("?c")),
		rewrite!("i-cos"; "(i (cos ?x) ?x)" => "(sin ?x)"),
		rewrite!("i-sin"; "(i (sin ?x) ?x)" => "(* -1 (cos ?x))"),
		rewrite!("i-sum"; "(i (+ ?f ?g) ?x)" => "(+ (i ?f ?x) (i ?g ?x))"),
		rewrite!("i-dif"; "(i (- ?f ?g) ?x)" => "(- (i ?f ?x) (i ?g ?x))"),
		rewrite!("i-parts"; "(i (* ?a ?b) ?x)" => "(- (* ?a (i ?b ?x)) (i (* (d ?x ?a) (i ?b ?x)) ?x))"),
	];

	rules.extend(rewrite!("zero-add"; "(+ ?a 0)" <=> "?a"));
	rules.extend(rewrite!("one-mul"; "(* ?a 1)" <=> "?a"));
	rules.extend(rewrite!("distribute"; "(* ?a (+ ?b ?c))" <=> "(+ (* ?a ?b) (* ?a ?c))"));
	rules.extend(rewrite!("pow2"; "(pow ?x 2)" <=> "(* ?x ?x)"));

	rules
}

fn is_const_or_distinct_var(v: &str, w: &str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
	let v = v.parse().unwrap();
	let w = w.parse().unwrap();
	move |egraph, _, subst| {
		egraph.find(subst[v]) != egraph.find(subst[w])
			&& (egraph[subst[v]].data.is_some()
				|| egraph[subst[v]]
					.nodes
					.iter()
					.any(|n| matches!(n, Math::Symbol(..))))
	}
}

fn is_const(var: &str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
	let var = var.parse().unwrap();
	move |egraph, _, subst| egraph[subst[var]].data.is_some()
}

fn is_sym(var: &str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
	let var = var.parse().unwrap();
	move |egraph, _, subst| {
		egraph[subst[var]]
			.nodes
			.iter()
			.any(|n| matches!(n, Math::Symbol(..)))
	}
}

fn is_not_zero(var: &str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
	let var = var.parse().unwrap();
	move |egraph, _, subst| {
		if let Some(n) = &egraph[subst[var]].data {
			*(n.0) != 0.0
		} else {
			true
		}
	}
}
