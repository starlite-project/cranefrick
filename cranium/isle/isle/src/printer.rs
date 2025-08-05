use std::{
	io::{self, prelude::*},
	slice, vec,
};

use super::ast::{
	Converter, Decl, Def, Expr, Extern, Extractor, Field, Form, Ident, IfLet, Instantiation,
	LetDef, Model, ModelType, ModelValue, Pattern, Rule, Signature, Spec, SpecExpr, SpecOp, Type,
	TypeValue, Variant,
};

struct Printer<'a, W: Write> {
	out: &'a mut W,
	column: usize,
	indent: usize,
	width: usize,
}

impl<'a, W: Write> Printer<'a, W> {
	const fn new(out: &'a mut W, width: usize) -> Self {
		Self {
			out,
			column: 0,
			indent: 0,
			width,
		}
	}

	fn print(&mut self, sexpr: &SExpr) -> io::Result<()> {
		self.print_wrapped(sexpr, Wrapping::Wrap)
	}

	fn print_wrapped(&mut self, sexpr: &SExpr, wrapping: Wrapping) -> io::Result<()> {
		match sexpr {
			SExpr::Atom(atom) => self.put(atom)?,
			SExpr::Binding(name, sexpr) => {
				self.put(name)?;
				self.put(" @ ")?;
				self.print_wrapped(sexpr, wrapping)?;
			}
			SExpr::List(items) => {
				if matches!(wrapping, Wrapping::SingleLine) || self.fits(sexpr) {
					self.put("(")?;
					for (i, item) in items.iter().enumerate() {
						if i > 0 {
							self.put(" ")?;
						}

						self.print_wrapped(item, Wrapping::SingleLine)?;
					}
					self.put(")")?;
				} else {
					let (first, rest) = items.split_first().expect("non-empty list");
					self.put("(")?;
					self.print_wrapped(first, wrapping)?;
					self.indent += 1;
					for item in rest {
						self.nl()?;
						self.print_wrapped(item, wrapping)?;
					}

					self.indent -= 1;
					self.nl()?;
					self.put(")")?;
				}
			}
		}

		Ok(())
	}

	fn fits(&self, sexpr: &SExpr) -> bool {
		let Some(mut remaining) = self.width.checked_sub(self.column) else {
			return false;
		};

		let mut stack = vec![sexpr];
		while let Some(sexpr) = stack.pop() {
			let consume = match sexpr {
				SExpr::Atom(atom) => atom.len(),
				SExpr::Binding(name, inner) => {
					stack.push(inner);
					name.len() + 3
				}
				SExpr::List(items) => {
					stack.extend(items.iter().rev());
					2 + items.len() - 1
				}
			};

			if consume > remaining {
				return false;
			}

			remaining -= consume;
		}

		true
	}

	fn put(&mut self, s: &str) -> io::Result<()> {
		write!(self.out, "{s}")?;
		self.column += s.len();
		Ok(())
	}

	fn nl(&mut self) -> io::Result<()> {
		writeln!(self.out)?;
		self.column = 0;
		for _ in 0..self.indent {
			write!(self.out, "    ")?;
		}

		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SExpr {
	Atom(String),
	Binding(String, Box<Self>),
	List(Vec<Self>),
}

impl SExpr {
	fn atom(atom: impl Into<String>) -> Self {
		Self::Atom(atom.into())
	}

	fn list(items: &[impl ToSExpr]) -> Self {
		Self::List(items.iter().map(ToSExpr::to_sexpr).collect())
	}

	fn tagged(tag: &str, items: &[impl ToSExpr]) -> Self {
		let mut parts = vec![Self::atom(tag)];
		parts.extend(items.iter().map(ToSExpr::to_sexpr));
		Self::List(parts)
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Wrapping {
	Wrap,
	SingleLine,
}

pub trait ToSExpr {
	fn to_sexpr(&self) -> SExpr;
}

impl ToSExpr for Def {
	fn to_sexpr(&self) -> SExpr {
		match self {
			Self::Pragma(pragma) => match *pragma {},
			Self::Type(ty) => ty.to_sexpr(),
			Self::Rule(rule) => rule.to_sexpr(),
			Self::Extractor(extractor) => extractor.to_sexpr(),
			Self::Decl(decl) => decl.to_sexpr(),
			Self::Spec(spec) => spec.to_sexpr(),
			Self::Model(model) => model.to_sexpr(),
			Self::Form(form) => form.to_sexpr(),
			Self::Instantiation(instantiation) => instantiation.to_sexpr(),
			Self::Extern(ext) => ext.to_sexpr(),
			Self::Converter(converter) => converter.to_sexpr(),
		}
	}
}

impl ToSExpr for Type {
	fn to_sexpr(&self) -> SExpr {
		let Self {
			name,
			ty,
			is_extern,
			is_nodebug,
			..
		} = self;

		let mut parts = vec![SExpr::atom("type"), name.to_sexpr()];
		if *is_extern {
			parts.push(SExpr::atom("extern"));
		}

		if *is_nodebug {
			parts.push(SExpr::atom("nodebug"));
		}

		parts.push(ty.to_sexpr());
		SExpr::List(parts)
	}
}

impl ToSExpr for Rule {
	fn to_sexpr(&self) -> SExpr {
		let Self {
			name,
			prio,
			pattern,
			if_lets,
			expr,
			..
		} = self;

		let mut parts = vec![SExpr::atom("rule")];
		if let Some(name) = name {
			parts.push(name.to_sexpr());
		}

		if let Some(prio) = prio {
			parts.push(SExpr::atom(prio.to_string()));
		}

		parts.push(pattern.to_sexpr());
		parts.extend(if_lets.iter().map(ToSExpr::to_sexpr));
		parts.push(expr.to_sexpr());
		SExpr::List(parts)
	}
}

impl ToSExpr for Extractor {
	fn to_sexpr(&self) -> SExpr {
		let Self {
			term,
			args,
			template,
			..
		} = self;
		let mut sig = vec![term.to_sexpr()];
		sig.extend(args.iter().map(ToSExpr::to_sexpr));

		SExpr::List(vec![
			SExpr::atom("extractor"),
			SExpr::List(sig),
			template.to_sexpr(),
		])
	}
}

impl ToSExpr for Decl {
	fn to_sexpr(&self) -> SExpr {
		let Self {
			term,
			arg_tys,
			ret_ty,
			pure,
			multi,
			partial,
			..
		} = self;

		let mut parts = vec![SExpr::atom("decl")];
		if *pure {
			parts.push(SExpr::atom("pure"));
		}

		if *multi {
			parts.push(SExpr::atom("multi"));
		}

		if *partial {
			parts.push(SExpr::atom("partial"));
		}

		parts.extend([term.to_sexpr(), SExpr::list(arg_tys), ret_ty.to_sexpr()]);

		SExpr::List(parts)
	}
}

impl ToSExpr for Spec {
	fn to_sexpr(&self) -> SExpr {
		let Self {
			term,
			args,
			provides,
			requires,
		} = self;

		let mut sig = vec![term.to_sexpr()];
		sig.extend(args.iter().map(ToSExpr::to_sexpr));

		let mut parts = vec![SExpr::atom("spec")];
		parts.push(SExpr::List(sig));

		if !provides.is_empty() {
			parts.push(SExpr::tagged("provide", provides));
		}

		if !requires.is_empty() {
			parts.push(SExpr::tagged("require", requires));
		}

		SExpr::List(parts)
	}
}

impl ToSExpr for Model {
	fn to_sexpr(&self) -> SExpr {
		let Self { name, value } = self;
		SExpr::List(vec![
			SExpr::atom("model"),
			name.to_sexpr(),
			value.to_sexpr(),
		])
	}
}

impl ToSExpr for Form {
	fn to_sexpr(&self) -> SExpr {
		let Self {
			name, signatures, ..
		} = self;

		let mut parts = vec![SExpr::atom("form"), name.to_sexpr()];
		parts.extend(signatures.iter().map(ToSExpr::to_sexpr));
		SExpr::List(parts)
	}
}

impl ToSExpr for Instantiation {
	fn to_sexpr(&self) -> SExpr {
		let Self {
			term,
			form,
			signatures,
			..
		} = self;

		let mut parts = vec![SExpr::atom("instantiate"), term.to_sexpr()];
		if let Some(form) = form {
			parts.push(form.to_sexpr());
		} else {
			parts.extend(signatures.iter().map(ToSExpr::to_sexpr));
		}

		SExpr::List(parts)
	}
}

impl ToSExpr for Extern {
	fn to_sexpr(&self) -> SExpr {
		match self {
			Self::Extractor {
				term,
				func,
				infallible,
				..
			} => {
				let mut parts = vec![SExpr::atom("extern"), SExpr::atom("extractor")];
				if *infallible {
					parts.push(SExpr::atom("infallible"));
				}

				parts.push(term.to_sexpr());
				parts.push(func.to_sexpr());
				SExpr::List(parts)
			}
			Self::Constructor { term, func, .. } => SExpr::List(vec![
				SExpr::atom("extern"),
				SExpr::atom("constructor"),
				term.to_sexpr(),
				func.to_sexpr(),
			]),
			Self::Const { name, ty, .. } => SExpr::List(vec![
				SExpr::atom("extern"),
				SExpr::atom("const"),
				SExpr::atom(format!("${}", name.0)),
				ty.to_sexpr(),
			]),
		}
	}
}

impl ToSExpr for Converter {
	fn to_sexpr(&self) -> SExpr {
		let Self {
			inner_ty,
			outer_ty,
			term,
			..
		} = self;

		SExpr::List(vec![
			SExpr::atom("convert"),
			inner_ty.to_sexpr(),
			outer_ty.to_sexpr(),
			term.to_sexpr(),
		])
	}
}

impl ToSExpr for TypeValue {
	fn to_sexpr(&self) -> SExpr {
		match self {
			Self::Primitive(name, ..) => {
				SExpr::List(vec![SExpr::atom("primitive"), name.to_sexpr()])
			}
			Self::Enum(variants, ..) => {
				let mut parts = vec![SExpr::atom("enum")];
				parts.extend(variants.iter().map(ToSExpr::to_sexpr));
				SExpr::List(parts)
			}
		}
	}
}

impl ToSExpr for Variant {
	fn to_sexpr(&self) -> SExpr {
		let Self { name, fields, .. } = self;

		let mut parts = vec![name.to_sexpr()];
		parts.extend(fields.iter().map(ToSExpr::to_sexpr));
		SExpr::List(parts)
	}
}

impl ToSExpr for Field {
	fn to_sexpr(&self) -> SExpr {
		let Self { name, ty, .. } = self;
		SExpr::List(vec![name.to_sexpr(), ty.to_sexpr()])
	}
}

impl ToSExpr for ModelValue {
	fn to_sexpr(&self) -> SExpr {
		match self {
			Self::TypeValue(mt) => SExpr::List(vec![SExpr::atom("type"), mt.to_sexpr()]),
			Self::EnumValues(enumerators) => {
				let mut parts = vec![SExpr::atom("enum")];
				for (variant, value) in enumerators {
					parts.push(SExpr::List(vec![variant.to_sexpr(), value.to_sexpr()]));
				}

				SExpr::List(parts)
			}
		}
	}
}

impl ToSExpr for ModelType {
	fn to_sexpr(&self) -> SExpr {
		match self {
			Self::Unit => SExpr::atom("Unit"),
			Self::Int => SExpr::atom("Int"),
			Self::Bool => SExpr::atom("Bool"),
			Self::BitVec(Some(size)) => {
				SExpr::List(vec![SExpr::atom("bv"), SExpr::atom(size.to_string())])
			}
			Self::BitVec(None) => SExpr::List(vec![SExpr::atom("bv")]),
		}
	}
}

impl ToSExpr for Signature {
	fn to_sexpr(&self) -> SExpr {
		let Self {
			args,
			ret,
			canonical,
			..
		} = self;

		SExpr::List(vec![
			SExpr::tagged("args", args),
			SExpr::tagged("ret", slice::from_ref(ret)),
			SExpr::tagged("canon", slice::from_ref(canonical)),
		])
	}
}

impl ToSExpr for SpecExpr {
	fn to_sexpr(&self) -> SExpr {
		match self {
			Self::ConstInt { value, .. } => SExpr::atom(value.to_string()),
			Self::ConstBitVec { value, width, .. } => SExpr::atom(if matches!(*width % 4, 0) {
				format!("#x{value:0width$x}", width = *width as usize / 4)
			} else {
				format!("#b{value:0width$b}", width = *width as usize)
			}),
			Self::ConstBool { value, .. } => SExpr::atom(if *value { "true" } else { "false" }),
			Self::ConstUnit { .. } => SExpr::List(Vec::new()),
			Self::Var { var, .. } => var.to_sexpr(),
			Self::Op { op, args, .. } => {
				let mut parts = vec![op.to_sexpr()];
				parts.extend(args.iter().map(ToSExpr::to_sexpr));
				SExpr::List(parts)
			}
			Self::Pair { left, right } => SExpr::List(vec![left.to_sexpr(), right.to_sexpr()]),
			Self::Enum { name } => SExpr::List(vec![name.to_sexpr()]),
		}
	}
}

impl ToSExpr for SpecOp {
	fn to_sexpr(&self) -> SExpr {
		SExpr::atom(match self {
			Self::Eq => "=",
			Self::And => "and",
			Self::Not => "not",
			Self::Imp => "=>",
			Self::Or => "or",
			Self::Lte => "<=",
			Self::Lt => "<",
			Self::Gte => ">=",
			Self::Gt => ">",
			Self::BVNot => "bvnot",
			Self::BVAnd => "bvand",
			Self::BVOr => "bvor",
			Self::BVXor => "bvxor",
			Self::BVNeg => "bvneg",
			Self::BVAdd => "bvadd",
			Self::BVSub => "bvsub",
			Self::BVMul => "bvmul",
			Self::BVUdiv => "bvudiv",
			Self::BVUrem => "bvurem",
			Self::BVSdiv => "bvsdiv",
			Self::BVSrem => "bvsrem",
			Self::BVShl => "bvshl",
			Self::BVLshr => "bvlshr",
			Self::BVAshr => "bvashr",
			Self::BVSaddo => "bvsaddo",
			Self::BVUle => "bvule",
			Self::BVUlt => "bvult",
			Self::BVUgt => "bvugt",
			Self::BVUge => "bvuge",
			Self::BVSlt => "bvslt",
			Self::BVSle => "bvsle",
			Self::BVSgt => "bvsgt",
			Self::BVSge => "bvsge",
			Self::Rotr => "rotr",
			Self::Rotl => "rotl",
			Self::Extract => "extract",
			Self::ZeroExt => "zero_ext",
			Self::SignExt => "sign_ext",
			Self::Concat => "concat",
			Self::ConvTo => "conv_to",
			Self::Int2BV => "int2bv",
			Self::WidthOf => "widthof",
			Self::If => "if",
			Self::Switch => "switch",
			Self::Popcnt => "popcnt",
			Self::Rev => "rev",
			Self::Cls => "cls",
			Self::Clz => "clz",
			Self::Subs => "subs",
			Self::BV2Int => "bv2int",
			Self::LoadEffect => "load_effect",
			Self::StoreEffect => "store_effect",
		})
	}
}

impl ToSExpr for Pattern {
	fn to_sexpr(&self) -> SExpr {
		match self {
			Self::Var {
				var: Ident(var, ..),
				..
			} => SExpr::atom(var),
			Self::Bind {
				var: Ident(var, ..),
				subpat,
				..
			} => SExpr::Binding(var.clone(), Box::new(subpat.to_sexpr())),
			Self::ConstInt { value, .. } => SExpr::atom(value.to_string()),
			Self::ConstBool { value, .. } => SExpr::atom(if *value { "true" } else { "false" }),
			Self::ConstPrim { value, .. } => SExpr::atom(format!("${}", value.0)),
			Self::Wildcard { .. } => SExpr::atom("_"),
			Self::Term { sym, args, .. } => {
				let mut parts = vec![sym.to_sexpr()];
				parts.extend(args.iter().map(ToSExpr::to_sexpr));
				SExpr::List(parts)
			}
			Self::And { subpats, .. } => {
				let mut parts = vec![SExpr::atom("and")];
				parts.extend(subpats.iter().map(ToSExpr::to_sexpr));
				SExpr::List(parts)
			}
			Self::MacroArg { .. } => unimplemented!("macro arguments are for internal use only"),
		}
	}
}

impl ToSExpr for IfLet {
	fn to_sexpr(&self) -> SExpr {
		let Self { pattern, expr, .. } = self;

		SExpr::List(vec![
			SExpr::atom("if-let"),
			pattern.to_sexpr(),
			expr.to_sexpr(),
		])
	}
}

impl ToSExpr for Expr {
	fn to_sexpr(&self) -> SExpr {
		match self {
			Self::Term { sym, args, .. } => {
				let mut parts = vec![sym.to_sexpr()];
				parts.extend(args.iter().map(ToSExpr::to_sexpr));
				SExpr::List(parts)
			}
			Self::Var { name, .. } => name.to_sexpr(),
			Self::ConstInt { value, .. } => SExpr::atom(value.to_string()),
			Self::ConstBool { value, .. } => SExpr::atom(if *value { "true" } else { "false" }),
			Self::ConstPrim { value, .. } => SExpr::atom(format!("${}", value.0)),
			Self::Let { defs, body, .. } => {
				let mut parts = vec![SExpr::atom("let")];
				parts.push(SExpr::list(defs));
				parts.push(body.to_sexpr());
				SExpr::List(parts)
			}
		}
	}
}

impl ToSExpr for LetDef {
	fn to_sexpr(&self) -> SExpr {
		let Self { var, ty, value, .. } = self;
		SExpr::List(vec![var.to_sexpr(), ty.to_sexpr(), value.to_sexpr()])
	}
}

impl ToSExpr for Ident {
	fn to_sexpr(&self) -> SExpr {
		let Self(name, ..) = self;
		SExpr::atom(name)
	}
}

pub fn print<W: Write>(defs: &[Def], width: usize, out: &mut W) -> io::Result<()> {
	for (i, def) in defs.iter().enumerate() {
		if i > 0 {
			writeln!(out)?;
		}

		print_node(def, width, out)?;
		writeln!(out)?;
	}

	Ok(())
}

pub fn print_node<N: ToSExpr, W: Write>(node: &N, width: usize, out: &mut W) -> io::Result<()> {
	let mut printer = Printer::new(out, width);
	let sexpr = node.to_sexpr();
	printer.print(&sexpr)
}
