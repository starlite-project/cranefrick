#![allow(clippy::wildcard_imports)]

use std::collections::HashSet;

use color_eyre::Result;
use egg::*;
use egg_test::run_expr;
use rayon::prelude::*;

type EGraph = egg::EGraph<Lambda, LambdaAnalysis>;
type Rewrite = egg::Rewrite<Lambda, LambdaAnalysis>;

define_language! {
	enum Lambda {
		Bool(bool),
		Num(i32),

		"var" = Var(Id),

		"+" = Add([Id; 2]),
		"=" = Eq([Id; 2]),

		"app" = App([Id; 2]),
		"lam" = Lambda([Id; 2]),
		"let" = Let([Id; 3]),
		"fix" = Fix([Id; 2]),

		"if" = If([Id; 3]),

		Symbol(egg::Symbol),
	}
}

impl Lambda {
	const fn num(&self) -> Option<i32> {
		let Self::Num(n) = self else {
			return None;
		};

		Some(*n)
	}
}

#[derive(Default)]
struct LambdaAnalysis;

impl Analysis<Lambda> for LambdaAnalysis {
	type Data = Data;

	fn merge(&mut self, to: &mut Self::Data, from: Self::Data) -> DidMerge {
		let before_len = to.free.len();
		to.free.retain(|i| from.free.contains(i));

		DidMerge(
			before_len != to.free.len(),
			to.free.len() != from.free.len(),
		) | merge_option(&mut to.constant, from.constant, |a, b| {
			assert_eq!(a.0, b.0, "merged non-equal constants");
			DidMerge(false, false)
		})
	}

	fn make(egraph: &mut EGraph, enode: &Lambda) -> Self::Data {
		let f = |i: &Id| egraph[*i].data.free.iter().copied();
		let mut free = HashSet::default();
		match enode {
			Lambda::Var(v) => {
				free.insert(*v);
			}
			Lambda::Let([v, a, b]) => {
				free.extend(f(b));
				free.remove(v);
				free.extend(f(a));
			}
			Lambda::Lambda([v, a]) | Lambda::Fix([v, a]) => {
				free.extend(f(a));
				free.remove(v);
			}
			_ => enode.for_each(|c| free.extend(&egraph[c].data.free)),
		}
		let constant = eval(egraph, enode);
		Data { free, constant }
	}

	fn modify(egraph: &mut EGraph, id: Id) {
		if let Some(c) = egraph[id].data.constant.clone() {
			if egraph.are_explanations_enabled() {
				egraph.union_instantiations(
					&c.0.to_string().parse().unwrap(),
					&c.1,
					&Subst::default(),
					"analysis".to_owned(),
				);
			} else {
				let const_id = egraph.add(c.0);
				egraph.union(id, const_id);
			}
		}
	}
}

#[derive(Debug)]
struct Data {
	free: HashSet<Id>,
	constant: Option<(Lambda, PatternAst<Lambda>)>,
}

fn main() -> Result<()> {
	color_eyre::install()?;

	let exprs = &[
		"(lam x (+ 4 (app (lam y (var y)) 4)))",
		"(if (= (var a) (var b)) (+ (var a) (var a)) (+ (var a) (var b)))",
		"(let x 0 (let y 1 (+ (var x) (var y))))",
		"(let compose (lam f (lam g (lam x (app (var f) (app (var g) (var x)))))) (let add1 (lam y (+ (var y) 1)) (app (app (var compose) (var add1)) (var add1))))",
		"(if (= 1 1) 7 9)",
		"(let compose (lam f (lam g (lam x (app (var f) (app (var g) (var x)))))) (let add1 (lam y (+ (var y) 1)) (app (app (var compose) (var add1)) (app (app (var compose) (var add1)) (app (app (var compose) (var add1)) (app (app (var compose) (var add1)) (app (app (var compose) (var add1)) (app (app (var compose) (var add1)) (var add1)))))))))",
		"(let compose (lam f (lam g (lam x (app (var f) (app (var g) (var x)))))) (let repeat (fix repeat (lam fun (lam n (if (= (var n) 0) (lam i (var i)) (app (app (var compose) (var fun)) (app (app (var repeat) (var fun)) (+ (var n) -1))))))) (let add1 (lam y (+ (var y) 1)) (app (app (var repeat) (var add1)) 2))))",
		"(let zeroone (lam x (if (= (var x) 0) 0 1)) (+ (app (var zeroone) 0) (app (var zeroone) 10)))",
		"(let fib (fix fib (lam n (if (= (var n) 0) 0 (if (= (var n) 1) 1 (+ (app (var fib) (+ (var n) -1)) (app (var fib) (+ (var n) -2))))))) (app (var fib) 4))",
		"(let zeroone (lam x (if (= (var x) 0) 0 1)) (+ (app (var zeroone) 0) (app (var zeroone) 10)))",
		"(let compose (lam f (lam g (lam x (app (var f) (app (var g) (var x)))))) (let repeat (fix repeat (lam fun (lam n (if (= (var n) 0) (lam i (var i)) (app (app (var compose) (var fun)) (app (app (var repeat) (var fun)) (+ (var n) -1))))))) (let add1 (lam y (+ (var y) 1)) (app (app (var repeat) (var add1)) 2))))",
		"(let fib (fix fib (lam n (if (= (var n) 0) 0 (if (= (var n) 1) 1 (+ (app (var fib) (+ (var n) -1)) (app (var fib) (+ (var n) -2))))))) (app (var fib) 4))",
		"(if (= (var ?x) ?e) ?then ?else)",
		"(+ (+ ?a ?b) ?c)",
		"(let ?v (fix ?v ?e) ?e)",
		"(app (lam ?v ?body) ?e)",
		"(let ?v ?e (app ?a ?b))",
		"(app (let ?v ?e ?a) (let ?v ?e ?b))",
		"(let ?v ?e (+   ?a ?b))",
		"(+   (let ?v ?e ?a) (let ?v ?e ?b))",
		"(let ?v ?e (=   ?a ?b))",
		"(=   (let ?v ?e ?a) (let ?v ?e ?b))",
		"(let ?v ?e (if ?cond ?then ?else))",
		"(if (let ?v ?e ?cond) (let ?v ?e ?then) (let ?v ?e ?else))",
		"(let ?v1 ?e (var ?v1))",
		"(let ?v1 ?e (var ?v2))",
		"(let ?v1 ?e (lam ?v1 ?body))",
		"(let ?v1 ?e (lam ?v2 ?body))",
		"(lam ?v2 (let ?v1 ?e ?body))",
		"(lam ?fresh (let ?v1 ?e (let ?v2 (var ?fresh) ?body)))",
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
	vec![
		rewrite!("if-true"; "(if true ?then ?else)" => "?then"),
		rewrite!("if-false"; "(if false ?then ?else)" => "?else"),
		rewrite!("if-elim"; "(if (= (var ?x) ?e) ?then ?else)" => "?else" if ConditionEqual::parse("(let ?x ?e ?then)", "(let ?x ?e ?else)")),
		rewrite!("add-comm"; "(+ ?a ?b)" => "(+ ?b ?a)"),
		rewrite!("add-assoc"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
		rewrite!("eq-comm"; "(= ?a ?b)" => "(= ?b ?a)"),
		rewrite!("fix"; "(fix ?v ?e)" => "(let ?v (fix ?v ?e) ?e)"),
		rewrite!("beta"; "(app (lam ?v ?body) ?e)" => "(let ?v ?e ?body)"),
		rewrite!("let-app"; "(let ?v ?e (app ?a ?b))" => "(app (let ?v ?e ?a) (let ?v ?e ?b))"),
		rewrite!("let-add"; "(let ?v ?e (+   ?a ?b))" => "(+   (let ?v ?e ?a) (let ?v ?e ?b))"),
		rewrite!("let-eq"; "(let ?v ?e (=   ?a ?b))" => "(=   (let ?v ?e ?a) (let ?v ?e ?b))"),
		rewrite!("let-const"; "(let ?v ?e ?c)" => "?c" if is_const(var("?c"))),
		rewrite!("let-if";
			"(let ?v ?e (if ?cond ?then ?else))" =>
			"(if (let ?v ?e ?cond) (let ?v ?e ?then) (let ?v ?e ?else))"
		),
		rewrite!("let-var-same"; "(let ?v1 ?e (var ?v1))" => "?e"),
		rewrite!("let-var-diff"; "(let ?v1 ?e (var ?v2))" => "(var ?v2)" if is_not_same_var(var("?v1"), var("?v2"))),
		rewrite!("let-lam-same"; "(let ?v1 ?e (lam ?v1 ?body))" => "(lam ?v1 ?body)"),
		rewrite!("let-lam-diff";
            "(let ?v1 ?e (lam ?v2 ?body))" =>
            { CaptureAvoid {
                fresh: var("?fresh"), v2: var("?v2"), e: var("?e"),
                if_not_free: "(lam ?v2 (let ?v1 ?e ?body))".parse().unwrap(),
                if_free: "(lam ?fresh (let ?v1 ?e (let ?v2 (var ?fresh) ?body)))".parse().unwrap(),
            }}
            if is_not_same_var(var("?v1"), var("?v2"))),
	]
}

fn eval(egraph: &EGraph, enode: &Lambda) -> Option<(Lambda, PatternAst<Lambda>)> {
	let x = |i: &Id| egraph[*i].data.constant.as_ref().map(|c| &c.0);
	match enode {
		Lambda::Num(n) => Some((enode.clone(), n.to_string().parse().ok()?)),
		Lambda::Bool(b) => Some((enode.clone(), b.to_string().parse().ok()?)),
		Lambda::Add([a, b]) => Some((
			Lambda::Num(x(a)?.num()?.checked_add(x(b)?.num()?)?),
			format!("(+ {} {})", x(a)?, x(b)?).parse().ok()?,
		)),
		Lambda::Eq([a, b]) => Some((
			Lambda::Bool(x(a)? == x(b)?),
			format!("(= {} {})", x(a)?, x(b)?).parse().ok()?,
		)),
		_ => None,
	}
}

fn var(s: &str) -> Var {
	s.parse().unwrap()
}

fn is_not_same_var(v1: Var, v2: Var) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
	move |egraph, _, subst| egraph.find(subst[v1]) != egraph.find(subst[v2])
}

fn is_const(v: Var) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
	move |egraph, _, subst| egraph[subst[v]].data.constant.is_some()
}

struct CaptureAvoid {
	fresh: Var,
	v2: Var,
	e: Var,
	if_not_free: Pattern<Lambda>,
	if_free: Pattern<Lambda>,
}

impl Applier<Lambda, LambdaAnalysis> for CaptureAvoid {
	fn apply_one(
		&self,
		egraph: &mut EGraph,
		eclass: Id,
		subst: &Subst,
		searcher_ast: Option<&PatternAst<Lambda>>,
		rule_name: Symbol,
	) -> Vec<Id> {
		let e = subst[self.e];
		let v2 = subst[self.v2];
		let v2_free_in_e = egraph[e].data.free.contains(&v2);
		if v2_free_in_e {
			let mut subst = subst.clone();
			let sym = Lambda::Symbol(format!("_{eclass}").into());
			subst.insert(self.fresh, egraph.add(sym));
			self.if_free
				.apply_one(egraph, eclass, &subst, searcher_ast, rule_name)
		} else {
			self.if_not_free
				.apply_one(egraph, eclass, subst, searcher_ast, rule_name)
		}
	}
}
