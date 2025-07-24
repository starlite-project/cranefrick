#![allow(clippy::wildcard_imports)]

use anyhow::Result;
use egg::*;

type EGraph = egg::EGraph<Prop, ConstantFold>;
type Rewrite = egg::Rewrite<Prop, ConstantFold>;

define_language! {
	enum Prop {
		Bool(bool),
		"&" = And([Id; 2]),
		"~" = Not(Id),
		"|" = Or([Id; 2]),
		"->" = Implies([Id; 2]),
		Symbol(Symbol),
	}
}

struct ConstantFold;

impl Analysis<Prop> for ConstantFold {
	type Data = Option<(bool, PatternAst<Prop>)>;

	fn merge(&mut self, a: &mut Self::Data, b: Self::Data) -> DidMerge {
		merge_option(a, b, |a, b| {
			assert_eq!(a.0, b.0, "merged non-equal constants");
			DidMerge(false, false)
		})
	}

	fn make(egraph: &mut EGraph, enode: &Prop) -> Self::Data {
		let x = |i: &Id| egraph[*i].data.as_ref().map(|c| c.0);
		let result = match enode {
			Prop::Bool(c) => Some((*c, c.to_string().parse().unwrap())),
			Prop::Symbol(_) => None,
			Prop::And([a, b]) => Some((
				x(a)? && x(b)?,
				format!("(& {} {})", x(a)?, x(b)?).parse().unwrap(),
			)),
			Prop::Not(a) => Some((!x(a)?, format!("(~ {})", x(a)?).parse().unwrap())),
			Prop::Or([a, b]) => Some((
				x(a)? || x(b)?,
				format!("(| {} {})", x(a)?, x(b)?).parse().unwrap(),
			)),
			Prop::Implies([a, b]) => Some((
				!x(a)? || x(b)?,
				format!("(-> {} {})", x(a)?, x(b)?).parse().unwrap(),
			)),
		};

		println!("make: {enode:?} -> {result:?}");
		result
	}
}

macro_rules! rule {
    ($name:ident, $left:literal, $right:literal) => {
        #[allow(dead_code)]
        fn $name() -> Rewrite {
            rewrite!(stringify!($name); $left => $right)
        }
    };
    ($name:ident, $name2:ident, $left:literal, $right:literal) => {
        rule!($name, $left, $right);
        rule!($name2, $right, $left);
    };
}

rule! {def_imply, def_imply_flip,   "(-> ?a ?b)",       "(| (~ ?a) ?b)"          }
rule! {double_neg, double_neg_flip,  "(~ (~ ?a))",       "?a"                     }
rule! {assoc_or,    "(| ?a (| ?b ?c))", "(| (| ?a ?b) ?c)"       }
rule! {dist_and_or, "(& ?a (| ?b ?c))", "(| (& ?a ?b) (& ?a ?c))"}
rule! {dist_or_and, "(| ?a (& ?b ?c))", "(& (| ?a ?b) (| ?a ?c))"}
rule! {comm_or,     "(| ?a ?b)",        "(| ?b ?a)"              }
rule! {comm_and,    "(& ?a ?b)",        "(& ?b ?a)"              }
rule! {lem,         "(| ?a (~ ?a))",    "true"                      }
rule! {or_true,     "(| ?a true)",         "true"                      }
rule! {and_true,    "(& ?a true)",         "?a"                     }
rule! {contrapositive, "(-> ?a ?b)",    "(-> (~ ?b) (~ ?a))"     }

fn lem_imply() -> Rewrite {
	multi_rewrite!(
		"lem_imply";
		"?value = true = (& (-> ?a ?b) (-> (~ ?a) ?c))"
		=>
		"?value = (| ?b ?c)"
	)
}

fn main() -> Result<()> {


	Ok(())
}
