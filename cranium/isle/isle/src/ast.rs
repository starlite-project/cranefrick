use super::lexer::Pos;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ident(pub String, pub Pos);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
	pub name: Ident,
	pub is_extern: bool,
	pub is_nodebug: bool,
	pub ty: TypeValue,
	pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
	pub name: Ident,
	pub fields: Vec<Field>,
	pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
	pub name: Ident,
	pub ty: Ident,
	pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decl {
	pub term: Ident,
	pub arg_tys: Vec<Ident>,
	pub ret_ty: Ident,
	pub pure: bool,
	pub multi: bool,
	pub partial: bool,
	pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
	pub term: Ident,
	pub args: Vec<Ident>,
	pub provides: Vec<SpecExpr>,
	pub requires: Vec<SpecExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
	pub name: Ident,
	pub value: ModelValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
	pub args: Vec<ModelType>,
	pub ret: ModelType,
	pub canonical: ModelType,
	pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
	pub name: Ident,
	pub signatures: Vec<Signature>,
	pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instantiation {
	pub term: Ident,
	pub form: Option<Ident>,
	pub signatures: Vec<Signature>,
	pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
	pub pattern: Pattern,
	pub if_lets: Vec<IfLet>,
	pub expr: Expr,
	pub pos: Pos,
	pub prio: Option<i64>,
	pub name: Option<Ident>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfLet {
	pub pattern: Pattern,
	pub expr: Expr,
	pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extractor {
	pub term: Ident,
	pub args: Vec<Ident>,
	pub template: Pattern,
	pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetDef {
	pub var: Ident,
	pub ty: Ident,
	pub value: Box<Expr>,
	pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converter {
	pub term: Ident,
	pub inner_ty: Ident,
	pub outer_ty: Ident,
	pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Def {
	Pragma(Pragma),
	Type(Type),
	Rule(Rule),
	Extractor(Extractor),
	Decl(Decl),
	Spec(Spec),
	Model(Model),
	Form(Form),
	Instantiation(Instantiation),
	Extern(Extern),
	Converter(Converter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pragma {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeValue {
	Primitive(Ident, Pos),
	Enum(Vec<Variant>, Pos),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecExpr {
	ConstInt {
		value: i128,
		pos: Pos,
	},
	ConstBitVec {
		value: i128,
		width: i8,
		pos: Pos,
	},
	ConstBool {
		value: bool,
		pos: Pos,
	},
	ConstUnit {
		pos: Pos,
	},
	Var {
		var: Ident,
		pos: Pos,
	},
	Op {
		op: SpecOp,
		args: Vec<Self>,
		pos: Pos,
	},
	Pair {
		left: Box<Self>,
		right: Box<Self>,
	},
	Enum {
		name: Ident,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecOp {
	Eq,
	And,
	Or,
	Not,
	Imp,
	Lt,
	Lte,
	Gt,
	Gte,
	BVNot,
	BVAnd,
	BVOr,
	BVXor,
	BVNeg,
	BVAdd,
	BVSub,
	BVMul,
	BVUdiv,
	BVUrem,
	BVSdiv,
	BVSrem,
	BVShl,
	BVLshr,
	BVAshr,
	BVUle,
	BVUlt,
	BVUgt,
	BVUge,
	BVSlt,
	BVSle,
	BVSgt,
	BVSge,
	BVSaddo,
	Rotr,
	Rotl,
	Extract,
	ZeroExt,
	SignExt,
	Concat,
	Subs,
	Popcnt,
	Clz,
	Cls,
	Rev,
	ConvTo,
	Int2BV,
	BV2Int,
	WidthOf,
	If,
	Switch,
	LoadEffect,
	StoreEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
	Int,
	Bool,
	BitVec(Option<usize>),
	Unit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelValue {
	TypeValue(ModelType),
	EnumValues(Vec<(Ident, SpecExpr)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
	Var {
		var: Ident,
		pos: Pos,
	},
	Bind {
		var: Ident,
		subpat: Box<Pattern>,
		pos: Pos,
	},
	ConstBool {
		value: bool,
		pos: Pos,
	},
	ConstInt {
		value: i128,
		pos: Pos,
	},
	ConstPrim {
		value: Ident,
		pos: Pos,
	},
	Term {
		sym: Ident,
		args: Vec<Self>,
		pos: Pos,
	},
	Wildcard {
		pos: Pos,
	},
	And {
		subpats: Vec<Pattern>,
		pos: Pos,
	},
	MacroArg {
		index: usize,
		pos: Pos,
	},
}

impl Pattern {
	#[must_use]
	pub const fn root_term(&self) -> Option<&Ident> {
		match self {
			Self::Term { sym, .. } => Some(sym),
			_ => None,
		}
	}

	pub fn terms(&self, f: &mut dyn FnMut(Pos, &Ident)) {
		match self {
			Self::Term { sym, args, pos } => {
				f(*pos, sym);
				args.iter().for_each(|arg| arg.terms(f));
			}
			Self::And { subpats, .. } => subpats.iter().for_each(|p| p.terms(f)),
			Self::Bind { subpat, .. } => subpat.terms(f),
			_ => {}
		}
	}

	#[tracing::instrument(level = "trace")]
	#[must_use]
	pub fn make_macro_template(&self, macro_args: &[Ident]) -> Self {
		match self {
			Self::Bind { var, subpat, pos } if matches!(&**subpat, Self::Wildcard { .. }) => {
				if let Some(i) = macro_args.iter().position(|arg| arg.0 == var.0) {
					Self::MacroArg {
						index: i,
						pos: *pos,
					}
				} else {
					self.clone()
				}
			}
			Self::Bind { var, subpat, pos } => Self::Bind {
				var: var.clone(),
				subpat: Box::new(subpat.make_macro_template(macro_args)),
				pos: *pos,
			},
			Self::Var { var, pos } => {
				if let Some(i) = macro_args.iter().position(|arg| arg.0 == var.0) {
					Self::MacroArg {
						index: i,
						pos: *pos,
					}
				} else {
					self.clone()
				}
			}
			Self::And { subpats, pos } => {
				let subpats = subpats
					.iter()
					.map(|subpat| subpat.make_macro_template(macro_args))
					.collect();
				Self::And { subpats, pos: *pos }
			}
			Self::Term { sym, args, pos } => {
				let args = args
					.iter()
					.map(|arg| arg.make_macro_template(macro_args))
					.collect();

				Self::Term {
					sym: sym.clone(),
					args,
					pos: *pos,
				}
			}
			Self::MacroArg { .. } => unreachable!(),
			_ => self.clone(),
		}
	}

	#[tracing::instrument(level = "trace")]
	pub fn subst_macro_args(&self, macro_args: &[Self]) -> Option<Self> {
		match self {
			Self::Bind { var, subpat, pos } => Some(Self::Bind {
				var: var.clone(),
				subpat: Box::new(subpat.subst_macro_args(macro_args)?),
				pos: *pos,
			}),
			Self::And { subpats, pos } => {
				let subpats = subpats
					.iter()
					.map(|subpat| subpat.subst_macro_args(macro_args))
					.collect::<Option<_>>()?;

				Some(Self::And { subpats, pos: *pos })
			}
			Self::Term { sym, args, pos } => {
				let args = args
					.iter()
					.map(|arg| arg.subst_macro_args(macro_args))
					.collect::<Option<_>>()?;

				Some(Self::Term {
					sym: sym.clone(),
					args,
					pos: *pos,
				})
			}
			Self::MacroArg { index, .. } => macro_args.get(*index).cloned(),
			_ => Some(self.clone()),
		}
	}

	#[must_use]
	pub const fn pos(&self) -> Pos {
		match self {
			Self::ConstBool { pos, .. }
			| Self::ConstInt { pos, .. }
			| Self::ConstPrim { pos, .. }
			| Self::And { pos, .. }
			| Self::Term { pos, .. }
			| Self::Bind { pos, .. }
			| Self::Var { pos, .. }
			| Self::Wildcard { pos }
			| Self::MacroArg { pos, .. } => *pos,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
	Term {
		sym: Ident,
		args: Vec<Self>,
		pos: Pos,
	},
	Var {
		name: Ident,
		pos: Pos,
	},
	ConstBool {
		value: bool,
		pos: Pos,
	},
	ConstInt {
		value: i128,
		pos: Pos,
	},
	ConstPrim {
		value: Ident,
		pos: Pos,
	},
	Let {
		defs: Vec<LetDef>,
		body: Box<Self>,
		pos: Pos,
	},
}

impl Expr {
	#[must_use]
	pub const fn pos(&self) -> Pos {
		match self {
			Self::Term { pos, .. }
			| Self::Var { pos, .. }
			| Self::ConstBool { pos, .. }
			| Self::ConstInt { pos, .. }
			| Self::ConstPrim { pos, .. }
			| Self::Let { pos, .. } => *pos,
		}
	}

	pub fn terms(&self, f: &mut dyn FnMut(Pos, &Ident)) {
		match self {
			Self::Term { sym, args, pos } => {
				f(*pos, sym);
				args.iter().for_each(|arg| arg.terms(f));
			}
			Self::Let { defs, body, .. } => {
				defs.iter().for_each(|def| def.value.terms(f));
				body.terms(f);
			}
			_ => {}
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Extern {
	Extractor {
		term: Ident,
		func: Ident,
		pos: Pos,
		infallible: bool,
	},
	Constructor {
		term: Ident,
		func: Ident,
		pos: Pos,
	},
	Const {
		name: Ident,
		ty: Ident,
		pos: Pos,
	},
}
