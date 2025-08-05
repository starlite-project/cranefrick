#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

pub mod annotation;

use std::{
	collections::HashMap,
	fmt::{Display, Formatter, Result as FmtResult, Write as _},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeContext {
	pub ty_vars: HashMap<Expr, u32>,
	pub ty_map: HashMap<u32, Type>,
	pub ty_values: HashMap<u32, i128>,
	pub bv_unknown_width_sets: HashMap<u32, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcreteInput {
	pub literal: String,
	pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcreteTest {
	pub term_name: String,
	pub args: Vec<ConcreteInput>,
	pub output: ConcreteInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoundVar {
	pub name: String,
	pub ty_var: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TermSignature {
	pub args: Vec<Type>,
	pub ret: Type,
	pub canonical_type: Option<Type>,
}

impl Display for TermSignature {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let args = self
			.args
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>()
			.join(" ");
		let canon = self
			.canonical_type
			.map(|c| format!("(canon {c})"))
			.unwrap_or_default();

		f.write_str("((args ")?;
		f.write_str(&args)?;
		f.write_str(") (ret ")?;
		Display::fmt(&self.ret, f)?;
		f.write_str(") ")?;
		f.write_str(&canon)?;
		f.write_char(')')
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
	BitVector(Option<usize>),
	Bool,
	Int,
	Unit,
}

impl Display for Type {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::BitVector(None) => f.write_str("bv"),
			Self::BitVector(Some(s)) => {
				f.write_str("(bv ")?;
				Display::fmt(&s, f)?;
				f.write_char(')')
			}
			Self::Bool => f.write_str("Bool"),
			Self::Int => f.write_str("Int"),
			Self::Unit => f.write_str("Unit"),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Terminal {
	Var(String),

	// Literal SMT value, for testing (plus type variable)
	Literal(String, u32),

	// Value, type variable
	Const(i128, u32),
	True,
	False,
	Wildcard(u32),
}

impl Display for Terminal {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Var(v) | Self::Literal(v, ..) => f.write_str(v),
			Self::Const(c, ..) => Display::fmt(&c, f),
			Self::True => f.write_str("true"),
			Self::False => f.write_str("false"),
			Self::Wildcard(..) => f.write_char('_'),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
	// Boolean operations
	Not,

	// Bitvector operations
	BVNeg,
	BVNot,
}

impl Display for UnaryOp {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str(match *self {
			Self::Not => "not",
			Self::BVNeg => "bvneg",
			Self::BVNot => "bvnot",
		})
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
	// Boolean operations
	And,
	Or,
	Imp,
	Eq,
	Lte,
	Lt,

	// Bitvector operations
	BVSgt,
	BVSgte,
	BVSlt,
	BVSlte,
	BVUgt,
	BVUgte,
	BVUlt,
	BVUlte,

	BVMul,
	BVUDiv,
	BVSDiv,
	BVAdd,
	BVSub,
	BVUrem,
	BVSrem,
	BVAnd,
	BVOr,
	BVXor,
	BVRotl,
	BVRotr,
	BVShl,
	BVShr,
	BVAShr,

	BVSaddo,
}

impl Display for BinaryOp {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str(match *self {
			Self::And => "and",
			Self::Or => "or",
			Self::Imp => "=>",
			Self::Eq => "=",
			Self::Lte => "<=",
			Self::Lt => "<",
			Self::BVSgt => "bvsgt",
			Self::BVSgte => "bvsgte",
			Self::BVSlt => "bvslt",
			Self::BVSlte => "bvslte",
			Self::BVUgt => "bvugt",
			Self::BVUgte => "bvugte",
			Self::BVUlt => "bvult",
			Self::BVUlte => "bvulte",
			Self::BVMul => "bvmul",
			Self::BVUDiv => "bvudiv",
			Self::BVSDiv => "bvsdiv",
			Self::BVAdd => "bvadd",
			Self::BVSub => "bvsub",
			Self::BVUrem => "bvurem",
			Self::BVSrem => "bvsrem",
			Self::BVAnd => "bvand",
			Self::BVOr => "bvor",
			Self::BVXor => "bvxor",
			Self::BVRotl => "rotl",
			Self::BVRotr => "rotr",
			Self::BVShl => "bvshl",
			Self::BVShr => "bvshr",
			Self::BVAShr => "bvashr",
			Self::BVSaddo => "bvsaddo",
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub enum Expr {
	// Terminal nodes
	Terminal(Terminal),

	// Opcode nodes
	Unary(UnaryOp, Box<Self>),
	Binary(BinaryOp, Box<Self>, Box<Self>),

	// Count leading zeros
	CLZ(Box<Self>),
	CLS(Box<Self>),
	Rev(Box<Self>),

	BVPopcnt(Box<Self>),

	BVSubs(Box<Self>, Box<Self>, Box<Self>),

	// ITE
	Conditional(Box<Self>, Box<Self>, Box<Self>),

	// Switch
	Switch(Box<Self>, Vec<(Self, Self)>),

	// Conversions
	// Extract specified bits
	BVExtract(usize, usize, Box<Self>),

	// Concat bitvectors
	BVConcat(Vec<Self>),

	// Convert integer to bitvector with that value
	BVIntToBV(usize, Box<Self>),

	// Convert bitvector to integer with that value
	BVToInt(Box<Self>),

	// Zero extend, with static or dynamic width
	BVZeroExtTo(usize, Box<Self>),
	BVZeroExtToVarWidth(Box<Self>, Box<Self>),

	// Sign extend, with static or dynamic width
	BVSignExtTo(usize, Box<Self>),
	BVSignExtToVarWidth(Box<Self>, Box<Self>),

	// Conversion to wider/narrower bits, without an explicit extend
	BVConvTo(Box<Self>, Box<Self>),

	WidthOf(Box<Self>),

	LoadEffect(Box<Self>, Box<Self>, Box<Self>),
	StoreEffect(Box<Self>, Box<Self>, Box<Self>, Box<Self>),
}

impl Display for Expr {
	#[allow(clippy::many_single_char_names)]
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Terminal(t) => Display::fmt(&t, f),
			Self::Unary(o, e) => {
				// write!(f, "({op} {e})")

				f.write_char('(')?;
				Display::fmt(&o, f)?;
				f.write_char(' ')?;
				Display::fmt(&e, f)?;
				f.write_char(')')
			}
			Self::Binary(o, x, y) => {
				// write!(f, "({op} {x} {y})")

				f.write_char('(')?;
				Display::fmt(&o, f)?;
				f.write_char(' ')?;
				Display::fmt(&x, f)?;
				f.write_char(' ')?;
				Display::fmt(&y, f)?;
				f.write_char(')')
			}
			Self::CLZ(e) => write!(f, "(clz {e})"),
			Self::CLS(e) => write!(f, "(cls {e})"),
			Self::Rev(e) => write!(f, "(rev {e})"),
			Self::BVPopcnt(e) => write!(f, "(popcnt {e})"),
			Self::BVSubs(t, x, y) => write!(f, "(subs {t} {x} {y})"),
			Self::Conditional(c, t, e) => write!(f, "(if {c} {t} {e})"),
			Self::Switch(m, cs) => {
				let cases: Vec<String> = cs.iter().map(|(c, m)| format!("({c} {m})")).collect();
				write!(f, "(switch {m} {})", cases.join(""))
			}
			Self::BVExtract(h, l, e) => write!(f, "(extract {h} {l} {e})"),
			Self::BVConcat(es) => {
				let vs: Vec<String> = es.iter().map(|v| format!("{v}")).collect();
				write!(f, "(concat {})", vs.join(""))
			}
			Self::BVIntToBV(t, e) => write!(f, "(int2bv {t} {e})"),
			Self::BVToInt(b) => write!(f, "(bv2int {b})"),
			Self::BVZeroExtTo(d, e) => write!(f, "(zero_ext {d} {e})"),
			Self::BVZeroExtToVarWidth(d, e) => write!(f, "(zero_ext {d} {e})"),
			Self::BVSignExtTo(d, e) => write!(f, "(sign_ext {d} {e})"),
			Self::BVSignExtToVarWidth(d, e) => write!(f, "(sign_ext {d} {e})"),
			Self::BVConvTo(x, y) => write!(f, "(conv_to {x} {y})"),
			Self::WidthOf(e) => write!(f, "(widthof {e})"),
			Self::LoadEffect(x, y, z) => write!(f, "(load_effect {x} {y} {z})"),
			Self::StoreEffect(w, x, y, z) => write!(f, "(store_effect {w} {x} {y} {z})"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationResult {
	InapplicableRule,
	Success,
	Failure,
	Unknown,
	NoDistinctModels,
}
