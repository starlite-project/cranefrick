use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundVar {
	pub name: String,
	pub ty: Option<Type>,
}

impl BoundVar {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			ty: None,
		}
	}

	pub fn with_ty(name: impl Into<String>, ty: Type) -> Self {
		Self {
			name: name.into(),
			ty: Some(ty),
		}
	}

	#[must_use]
	pub fn as_expr(&self) -> Expr {
		Expr::var(self.name.as_str())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermSignature {
	pub args: Vec<BoundVar>,
	pub ret: BoundVar,
}

#[allow(clippy::vec_box)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermAnnotation {
	pub sig: TermSignature,
	pub assumptions: Vec<Box<Expr>>,
	pub assertions: Vec<Box<Expr>>,
}

impl TermAnnotation {
	pub fn new(
		sig: TermSignature,
		assumptions: impl IntoIterator<Item = Expr>,
		assertions: impl IntoIterator<Item = Expr>,
	) -> Self {
		Self {
			sig,
			assumptions: assumptions.into_iter().map(Box::new).collect(),
			assertions: assertions.into_iter().map(Box::new).collect(),
		}
	}

	#[must_use]
	pub const fn sig(&self) -> &TermSignature {
		&self.sig
	}

    #[must_use]
	pub fn assertions(&self) -> Vec<Expr> {
		self.assumptions.iter().map(|x| *x.clone()).collect()
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Const {
	pub ty: Type,
	pub value: i128,
	pub width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
	Poly(u32),
	BitVector,
	BitVectorWithWidth(usize),
	BitVectorUnknown(u32),
	Int,
	Bool,
	Unit,
}

impl Type {
	#[must_use]
	pub const fn is_poly(self) -> bool {
		matches!(self, Self::Poly(..))
	}
}

impl Display for Type {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Poly(..) => f.write_str("poly"),
			Self::BitVectorUnknown(..) | Self::BitVector => f.write_str("bv"),
			Self::BitVectorWithWidth(w) => {
				f.write_str("bv")?;
				Display::fmt(&w, f)
			}
			Self::Int => f.write_str("Int"),
			Self::Bool => f.write_str("Bool"),
			Self::Unit => f.write_str("Unit"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
	Const(usize),
	RegWidth,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
	Var(String),
	Const(Const),
	True,
	False,

	// Get the width of a bitvector
	WidthOf(Box<Self>),

	// Boolean operations
	Not(Box<Self>),
	And(Box<Self>, Box<Self>),
	Or(Box<Self>, Box<Self>),
	Imp(Box<Self>, Box<Self>),
	Eq(Box<Self>, Box<Self>),
	Lte(Box<Self>, Box<Self>),
	Lt(Box<Self>, Box<Self>),

	BVSgt(Box<Self>, Box<Self>),
	BVSgte(Box<Self>, Box<Self>),
	BVSlt(Box<Self>, Box<Self>),
	BVSlte(Box<Self>, Box<Self>),
	BVUgt(Box<Self>, Box<Self>),
	BVUgte(Box<Self>, Box<Self>),
	BVUlt(Box<Self>, Box<Self>),
	BVUlte(Box<Self>, Box<Self>),

	BVSaddo(Box<Self>, Box<Self>),

	// Bitvector operations
	//      Note: these follow the naming conventions of the SMT theory of bitvectors:
	//      https://SMT-LIB.cs.uiowa.edu/version1/logics/QF_BV.smt
	// Unary operators
	BVNeg(Box<Self>),
	BVNot(Box<Self>),
	CLZ(Box<Self>),
	CLS(Box<Self>),
	Rev(Box<Self>),
	BVPopcnt(Box<Self>),

	// Binary operators
	BVMul(Box<Self>, Box<Self>),
	BVUDiv(Box<Self>, Box<Self>),
	BVSDiv(Box<Self>, Box<Self>),
	BVAdd(Box<Self>, Box<Self>),
	BVSub(Box<Self>, Box<Self>),
	BVUrem(Box<Self>, Box<Self>),
	BVSrem(Box<Self>, Box<Self>),
	BVAnd(Box<Self>, Box<Self>),
	BVOr(Box<Self>, Box<Self>),
	BVXor(Box<Self>, Box<Self>),
	BVRotl(Box<Self>, Box<Self>),
	BVRotr(Box<Self>, Box<Self>),
	BVShl(Box<Self>, Box<Self>),
	BVShr(Box<Self>, Box<Self>),
	BVAShr(Box<Self>, Box<Self>),

	// Includes type
	BVSubs(Box<Self>, Box<Self>, Box<Self>),

	// Conversions
	// Zero extend, static and dynamic width
	BVZeroExtTo(Box<Width>, Box<Self>),
	BVZeroExtToVarWidth(Box<Self>, Box<Self>),

	// Sign extend, static and dynamic width
	BVSignExtTo(Box<Width>, Box<Self>),
	BVSignExtToVarWidth(Box<Self>, Box<Self>),

	// Extract specified bits
	BVExtract(usize, usize, Box<Self>),

	// Concat two bitvectors
	BVConcat(Vec<Self>),

	// Convert integer to bitvector
	BVIntToBv(usize, Box<Self>),

	// Convert bitvector to integer
	BVToInt(Box<Self>),

	// Conversion to wider/narrower bits, without an explicit extend
	// Allow the destination width to be symbolic.
	BVConvTo(Box<Self>, Box<Self>),

	// Conditional if-then-else
	Conditional(Box<Self>, Box<Self>, Box<Self>),

	// Switch
	Switch(Box<Self>, Vec<(Self, Self)>),

	LoadEffect(Box<Self>, Box<Self>, Box<Self>),

	StoreEffect(Box<Self>, Box<Self>, Box<Self>, Box<Self>),
}

impl Expr {
	pub fn var(s: impl Into<String>) -> Self {
		Self::Var(s.into())
	}

	pub fn unary(f: impl FnOnce(Box<Self>) -> Self, x: Self) -> Self {
		f(Box::new(x))
	}

	pub fn binary(f: impl FnOnce(Box<Self>, Box<Self>) -> Self, x: Self, y: Self) -> Self {
		f(Box::new(x), Box::new(y))
	}
}
