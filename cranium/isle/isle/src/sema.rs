use std::{
	borrow::Cow,
	collections::{BTreeMap, BTreeSet, HashMap, hash_map::Entry},
	fmt::{Display, Formatter, Result as FmtResult},
	mem,
};

use tracing::trace;

use super::{
	ast, declare_id,
	error::*,
	lexer::Pos,
	stable_mapset::{StableMap, StableSet},
};

declare_id!(Sym);

declare_id!(TypeId);

impl TypeId {
	pub const BOOL: Self = Self::builtin(BuiltinType::Bool);
	pub const I128: Self = Self::builtin(BuiltinType::Int(IntType::I128));
	pub const I16: Self = Self::builtin(BuiltinType::Int(IntType::I16));
	pub const I32: Self = Self::builtin(BuiltinType::Int(IntType::I32));
	pub const I64: Self = Self::builtin(BuiltinType::Int(IntType::I64));
	pub const I8: Self = Self::builtin(BuiltinType::Int(IntType::I8));
	pub const ISIZE: Self = Self::builtin(BuiltinType::Int(IntType::Isize));
	pub const U128: Self = Self::builtin(BuiltinType::Int(IntType::U128));
	pub const U16: Self = Self::builtin(BuiltinType::Int(IntType::U16));
	pub const U32: Self = Self::builtin(BuiltinType::Int(IntType::U32));
	pub const U64: Self = Self::builtin(BuiltinType::Int(IntType::U64));
	pub const U8: Self = Self::builtin(BuiltinType::Int(IntType::U8));
	pub const USIZE: Self = Self::builtin(BuiltinType::Int(IntType::Usize));

	const fn builtin(builtin: BuiltinType) -> Self {
		Self(builtin.to_usize())
	}
}

declare_id!(VariantId);

declare_id!(FieldId);

declare_id!(TermId);

declare_id!(RuleId);

declare_id!(VarId);

macro_rules! unwrap_or_continue {
	($e:expr) => {
		match $e {
			::core::option::Option::Some(x) => x,
			::core::option::Option::None => continue,
		}
	};
}

#[derive(Debug)]
pub struct TypeEnv {
	pub syms: Vec<String>,
	pub sym_map: StableMap<String, Sym>,
	pub types: Vec<Type>,
	pub type_map: StableMap<Sym, TypeId>,
	pub const_types: StableMap<Sym, TypeId>,
	pub errors: Vec<Error>,
}

impl TypeEnv {
	pub fn try_from_ast(defs: &[ast::Def]) -> Result<Self, Vec<Error>> {
		let mut ty_env = Self::default();

		for def in defs {
			if let ast::Def::Type(td) = def {
				let tid = TypeId(ty_env.type_map.len());
				let name = ty_env.intern_mut(&td.name);

				if let Some(existing) = ty_env.type_map.get(&name).copied() {
					ty_env.report_error(
						td.pos,
						format!("type with name '{}' defined more than once", td.name.0),
					);
					let pos = unwrap_or_continue!(ty_env.types.get(existing.index())).pos();
					match pos {
						Some(pos) => ty_env.report_error(
							pos,
							format!("type with name '{}' already defined here", td.name.0),
						),
						None => ty_env.report_error(
							td.pos,
							format!("type with name '{}' is a built-in type", td.name.0),
						),
					}
					continue;
				}

				ty_env.type_map.insert(name, tid);
			}
		}

		for def in defs {
			if let ast::Def::Type(td) = def {
				let tid = ty_env.types.len();
				if let Some(ty) = ty_env.type_from_ast(TypeId(tid), td) {
					ty_env.types.push(ty);
				}
			}
		}

		for def in defs {
			if let ast::Def::Extern(ast::Extern::Const { name, ty, pos }) = def {
				let Some(ty) = ty_env.get_type_by_name(ty) else {
					ty_env.report_error(*pos, "unknown type for constant");
					continue;
				};

				let name = ty_env.intern_mut(name);
				ty_env.const_types.insert(name, ty);
			}
		}

		ty_env.return_errors()?;

		Ok(ty_env)
	}

	fn return_errors(&mut self) -> Result<(), Vec<Error>> {
		if self.errors.is_empty() {
			Ok(())
		} else {
			Err(mem::take(&mut self.errors))
		}
	}

	#[allow(clippy::unused_self)]
	fn error(&self, pos: Pos, message: impl Into<String>) -> Error {
		Error::Type {
			message: message.into(),
			span: Span::from_single(pos),
		}
	}

	fn report_error(&mut self, pos: Pos, message: impl Into<String>) {
		let err = self.error(pos, message);
		self.errors.push(err);
	}

	fn intern_mut(&mut self, ident: &ast::Ident) -> Sym {
		if let Some(s) = self.sym_map.get(&ident.0).copied() {
			s
		} else {
			let s = Sym(self.syms.len());
			self.syms.push(ident.0.clone());
			self.sym_map.insert(ident.0.clone(), s);
			s
		}
	}

	fn intern(&self, ident: &ast::Ident) -> Option<Sym> {
		self.sym_map.get(&ident.0).copied()
	}

	fn type_from_ast(&mut self, tid: TypeId, ty: &ast::Type) -> Option<Type> {
		let name = self.intern(&ty.name).unwrap();

		match &ty.ty {
			ast::TypeValue::Primitive(id, ..) => {
				if ty.is_nodebug {
					self.report_error(ty.pos, "primitive types cannot be marked `nodebug`");
					return None;
				}
				if ty.is_extern {
					self.report_error(ty.pos, "primitive types cannot be marked `extern`");
					return None;
				}

				Some(Type::Primitive(tid, self.intern_mut(id), ty.pos))
			}
			ast::TypeValue::Enum(ty_variants, ..) => {
				if ty.is_extern && ty.is_nodebug {
					self.report_error(ty.pos, "external types cannot be marked `nodebug`");
					return None;
				}

				let mut variants = Vec::new();
				for variant in ty_variants {
					let combined_ident =
						ast::Ident(format!("{}.{}", ty.name.0, variant.name.0), variant.name.1);
					let fullname = self.intern_mut(&variant.name);
					let id = VariantId(variants.len());
					if variants.iter().any(|v: &Variant| v.name == name) {
						self.report_error(
							variant.pos,
							format!("duplicate variant name in type: '{}'", variant.name.0),
						);
						return None;
					}

					let mut fields = Vec::new();
					for field in &variant.fields {
						let field_name = self.intern_mut(&field.name);
						if fields.iter().any(|f: &Field| f.name == field_name) {
							self.report_error(
								field.pos,
								format!(
									"duplicate field name '{}' in variant '{}' of type",
									field.name.0, variant.name.0
								),
							);
							return None;
						}

						let Some(field_tid) = self.get_type_by_name(&field.ty) else {
							self.report_error(
								field.ty.1,
								format!(
									"unknown type '{}' for field '{}' in variant '{}'",
									field.ty.0, field.name.0, variant.name.0
								),
							);
							return None;
						};

						fields.push(Field {
							name: field_name,
							id: FieldId(fields.len()),
							ty: field_tid,
						});
					}

					variants.push(Variant {
						name,
						fullname,
						id,
						fields,
					});
				}

				Some(Type::Enum {
					name,
					id: tid,
					is_extern: ty.is_extern,
					is_nodebug: ty.is_nodebug,
					variants,
					pos: ty.pos,
				})
			}
		}
	}

	#[must_use]
	pub fn get_type_by_name(&self, sym: &ast::Ident) -> Option<TypeId> {
		self.intern(sym)
			.and_then(|sym| self.type_map.get(&sym))
			.copied()
	}
}

impl Default for TypeEnv {
	fn default() -> Self {
		Self {
			syms: BuiltinType::ALL.iter().map(ToString::to_string).collect(),
			sym_map: BuiltinType::ALL
				.iter()
				.enumerate()
				.map(|(idx, bt)| (bt.to_string(), Sym(idx)))
				.collect(),
			types: BuiltinType::ALL
				.iter()
				.map(|bt| Type::Builtin(*bt))
				.collect(),
			type_map: BuiltinType::ALL
				.iter()
				.enumerate()
				.map(|(idx, ..)| (Sym(idx), TypeId(idx)))
				.collect(),
			const_types: StableMap::new(),
			errors: Vec::new(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
	pub name: Sym,
	pub fullname: Sym,
	pub id: VariantId,
	pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
	pub name: Sym,
	pub id: FieldId,
	pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct TermEnv {
	pub terms: Vec<Term>,
	pub term_map: StableMap<Sym, TermId>,
	pub rules: Vec<Rule>,
	pub converters: StableMap<(TypeId, TypeId), TermId>,
	pub expand_internal_extractors: bool,
}

impl TermEnv {
	pub fn try_from_ast(
		ty_env: &mut TypeEnv,
		defs: &[ast::Def],
		expand_internal_extractors: bool,
	) -> Result<Self, Vec<Error>> {
		let mut env = Self {
			terms: Vec::new(),
			term_map: StableMap::new(),
			rules: Vec::new(),
			converters: StableMap::new(),
			expand_internal_extractors,
		};

		env.collect_pragmas(defs);
		env.collect_term_sigs(ty_env, defs);
		env.collect_enum_variant_terms(ty_env);
		ty_env.return_errors()?;

		Ok(env)
	}

	#[allow(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
	const fn collect_pragmas(&mut self, _: &[ast::Def]) {}

	fn collect_term_sigs(&mut self, ty_env: &mut TypeEnv, defs: &[ast::Def]) {
		for def in defs {
			if let ast::Def::Decl(decl) = def {
				let name = ty_env.intern_mut(&decl.term);
				if let Some(tid) = self.term_map.get(&name) {
					ty_env.report_error(decl.pos, format!("duplicate decl for '{}'", decl.term.0));

					ty_env.report_error(
						self.terms[tid.index()].decl_pos,
						format!("duplicate decl for '{}'", decl.term.0),
					);
				}

				if decl.multi && decl.partial {
					ty_env.report_error(
						decl.pos,
						format!("term '{}' can't be both multi and partial", decl.term.0),
					);
				}

				let Ok(arg_tys) = decl
					.arg_tys
					.iter()
					.map(|id| {
						ty_env.get_type_by_name(id).ok_or_else(|| {
							ty_env.report_error(id.1, format!("unknown arg type: '{}'", id.0));
						})
					})
					.collect::<Result<Vec<_>, _>>()
				else {
					continue;
				};

				let Some(ret_ty) = ty_env.get_type_by_name(&decl.ret_ty) else {
					ty_env.report_error(
						decl.ret_ty.1,
						format!("unknown return type: '{}'", decl.ret_ty.0),
					);
					continue;
				};

				let tid = TermId(self.terms.len());
				self.term_map.insert(name, tid);
				let flags = TermFlags {
					pure: decl.pure,
					multi: decl.multi,
					partial: decl.partial,
				};

				self.terms.push(Term {
					id: tid,
					decl_pos: decl.pos,
					name,
					arg_tys,
					ret_ty,
					kind: TermKind::Decl {
						flags,
						constructor_kind: None,
						extractor_kind: None,
					},
				});
			}
		}
	}

	fn collect_enum_variant_terms(&mut self, ty_env: &mut TypeEnv) {
		'types: for i in 0..ty_env.types.len() {
			let ty = &ty_env.types[i];

			if let Type::Enum {
				pos, id, variants, ..
			} = ty
			{
				for variant in variants {
					if self.term_map.contains_key(&variant.fullname) {
						let variant_name = ty_env.syms[variant.fullname.index()].clone();
						ty_env.report_error(
							*pos,
							format!("duplicate enum variant constructor: '{variant_name}'"),
						);
						continue 'types;
					}

					let tid = TermId(self.terms.len());
					let arg_tys = variant.fields.iter().map(|fld| fld.ty).collect();
					let ret_ty = id;
					self.terms.push(Term {
						id: tid,
						decl_pos: *pos,
						name: variant.fullname,
						arg_tys,
						ret_ty: *ret_ty,
						kind: TermKind::EnumVariant {
							variant: variant.id,
						},
					});

					self.term_map.insert(variant.fullname, tid);
				}
			}
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
	pub id: TermId,
	pub decl_pos: Pos,
	pub name: Sym,
	pub arg_tys: Vec<TypeId>,
	pub ret_ty: TypeId,
	pub kind: TermKind,
}

impl Term {
	#[must_use]
	pub const fn ty(&self) -> TypeId {
		self.ret_ty
	}

	fn check_args_count<T>(&self, args: &[T], ty_env: &mut TypeEnv, pos: Pos, sym: &ast::Ident) {
		if self.arg_tys.len() != args.len() {
			ty_env.report_error(
				pos,
				format!(
					"incorrect argument count for term '{}': got {}, expected {}",
					sym.0,
					args.len(),
					self.arg_tys.len()
				),
			);
		}
	}

	#[must_use]
	pub const fn is_enum_variant(&self) -> bool {
		matches!(self.kind, TermKind::EnumVariant { .. })
	}

	#[must_use]
	pub const fn is_partial(&self) -> bool {
		matches!(
			self.kind,
			TermKind::Decl {
				flags: TermFlags { partial: true, .. },
				..
			}
		)
	}

	#[must_use]
	pub const fn has_constructor(&self) -> bool {
		matches!(
			self.kind,
			TermKind::EnumVariant { .. }
				| TermKind::Decl {
					constructor_kind: Some(..),
					..
				}
		)
	}

	#[must_use]
	pub const fn has_extractor(&self) -> bool {
		matches!(
			self.kind,
			TermKind::EnumVariant { .. }
				| TermKind::Decl {
					extractor_kind: Some(..),
					..
				}
		)
	}

	#[must_use]
	pub const fn has_external_extractor(&self) -> bool {
		matches!(
			self.kind,
			TermKind::Decl {
				extractor_kind: Some(ExtractorKind::External { .. }),
				..
			}
		)
	}

	#[must_use]
	pub const fn has_external_constructor(&self) -> bool {
		matches!(
			self.kind,
			TermKind::Decl {
				constructor_kind: Some(ConstructorKind::External { .. }),
				..
			}
		)
	}

	#[must_use]
	pub fn extractor_sig(&self, ty_env: &TypeEnv) -> Option<ExternalSig> {
		match self.kind {
			TermKind::Decl {
				flags,
				extractor_kind: Some(ExtractorKind::External {
					name, infallible, ..
				}),
				..
			} => {
				let ret_kind = if flags.multi {
					ReturnKind::Iterator
				} else if infallible {
					ReturnKind::Plain
				} else {
					ReturnKind::Option
				};

				Some(ExternalSig {
					func_name: ty_env.syms[name.index()].clone(),
					full_name: format!("C::{}", ty_env.syms[name.index()]),
					param_tys: vec![self.ret_ty],
					ret_tys: self.arg_tys.clone(),
					ret_kind,
				})
			}
			_ => None,
		}
	}

	#[must_use]
	pub fn constructor_sig(&self, ty_env: &TypeEnv) -> Option<ExternalSig> {
		match self.kind {
			TermKind::Decl {
				constructor_kind: Some(kind),
				flags,
				..
			} => {
				let (func_name, full_name) = match kind {
					ConstructorKind::Internal => {
						let name = format!("constructor_{}", ty_env.syms[self.name.index()]);
						(name.clone(), name)
					}
					ConstructorKind::External { name } => (
						ty_env.syms[name.index()].clone(),
						format!("C::{}", ty_env.syms[name.index()]),
					),
				};

				let ret_kind = if flags.multi {
					ReturnKind::Iterator
				} else if flags.partial {
					ReturnKind::Option
				} else {
					ReturnKind::Plain
				};

				Some(ExternalSig {
					func_name,
					full_name,
					param_tys: self.arg_tys.clone(),
					ret_tys: vec![self.ret_ty],
					ret_kind,
				})
			}
			_ => None,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermFlags {
	pub pure: bool,
	pub multi: bool,
	pub partial: bool,
}

impl TermFlags {
	#[must_use]
	pub const fn on_lhs(mut self) -> Self {
		self.partial = true;
		self.pure = true;
		self
	}
}

#[derive(Debug, Clone)]
pub struct ExternalSig {
	pub func_name: String,
	pub full_name: String,
	pub param_tys: Vec<TypeId>,
	pub ret_tys: Vec<TypeId>,
	pub ret_kind: ReturnKind,
}

#[derive(Debug, Clone)]
pub struct Rule {
	pub id: RuleId,
	pub root_term: TermId,
	pub args: Vec<Pattern>,
	pub if_lets: Vec<IfLet>,
	pub rhs: Expr,
	pub vars: Vec<BoundVar>,
	pub prio: i64,
	pub pos: Pos,
	pub name: Option<Sym>,
}

impl Rule {
	pub fn visit<V: RuleVisitor>(&self, visitor: &mut V, term_env: &TermEnv) -> V::Expr {
		let mut vars = HashMap::new();

		let term_data = &term_env.terms[self.root_term.index()];
		for (i, (subpat, &arg_ty)) in self.args.iter().zip(term_data.arg_tys.iter()).enumerate() {
			let value = visitor.add_arg(i, arg_ty);
			visitor.add_pattern(|visitor| subpat.visit(visitor, value, term_env, &mut vars));
		}

		for if_let in &self.if_lets {
			let subexpr = if_let.rhs.visit_in_rule(visitor, term_env, &vars);
			let value = visitor.expr_as_pattern(subexpr);
			visitor.add_pattern(|visitor| if_let.lhs.visit(visitor, value, term_env, &mut vars));
		}

		self.rhs.visit_in_rule(visitor, term_env, &vars)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct BoundVar {
	pub id: VarId,
	pub name: Sym,
	pub ty: TypeId,
	scope: usize,
}

#[derive(Debug, Clone)]
pub struct IfLet {
	pub lhs: Pattern,
	pub rhs: Expr,
}

#[derive(Clone, Copy)]
pub struct VisitedExpr<V: ExprVisitor> {
	pub ty: TypeId,
	pub value: V::ExprId,
}

#[derive(Debug, Default, Clone)]
struct Bindings {
	seen: Vec<BoundVar>,
	next_scope: usize,
	in_scope: Vec<usize>,
}

impl Bindings {
	fn enter_scope(&mut self) {
		self.in_scope.push(self.next_scope);
		self.next_scope += 1;
	}

	fn exit_scope(&mut self) {
		self.in_scope.pop();
	}

	fn add_var(&mut self, name: Sym, ty: TypeId) -> VarId {
		let id = VarId(self.seen.len());
		let var = BoundVar {
			id,
			name,
			ty,
			scope: *self
				.in_scope
				.last()
				.expect("enter_scope should be called before add_var"),
		};

		trace!(var = ?var, "binding var");
		self.seen.push(var);
		id
	}

	fn lookup(&self, name: Sym) -> Option<&BoundVar> {
		self.seen
			.iter()
			.rev()
			.find(|binding| binding.name == name && self.in_scope.contains(&binding.scope))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinType {
	Bool,
	Int(IntType),
}

impl BuiltinType {
	pub const ALL: &'static [Self] = &[
		Self::Bool,
		Self::Int(IntType::U8),
		Self::Int(IntType::U16),
		Self::Int(IntType::U32),
		Self::Int(IntType::U64),
		Self::Int(IntType::U128),
		Self::Int(IntType::Usize),
		Self::Int(IntType::I8),
		Self::Int(IntType::I16),
		Self::Int(IntType::I32),
		Self::Int(IntType::I64),
		Self::Int(IntType::I128),
		Self::Int(IntType::Isize),
	];

	const fn to_usize(self) -> usize {
		match self {
			Self::Bool => 0,
			Self::Int(ty) => ty as usize + 1,
		}
	}
}

impl Display for BuiltinType {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Bool => f.write_str("bool"),
			Self::Int(i) => Display::fmt(&i, f),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntType {
	U8,
	U16,
	U32,
	U64,
	U128,
	Usize,
	I8,
	I16,
	I32,
	I64,
	I128,
	Isize,
}

impl IntType {
	#[must_use]
	pub const fn is_signed(self) -> bool {
		matches!(
			self,
			Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::I128 | Self::Isize
		)
	}
}

impl Display for IntType {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str(match self {
			Self::U8 => "u8",
			Self::U16 => "u16",
			Self::U32 => "u32",
			Self::U64 => "u64",
			Self::U128 => "u128",
			Self::Usize => "usize",
			Self::I8 => "i8",
			Self::I16 => "i16",
			Self::I32 => "i32",
			Self::I64 => "i64",
			Self::I128 => "i128",
			Self::Isize => "isize",
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
	Builtin(BuiltinType),
	Primitive(TypeId, Sym, Pos),
	Enum {
		name: Sym,
		id: TypeId,
		is_extern: bool,
		is_nodebug: bool,
		variants: Vec<Variant>,
		pos: Pos,
	},
}

impl Type {
	#[must_use]
	pub fn name<'a>(&self, ty_env: &'a TypeEnv) -> Cow<'a, str> {
		match self {
			Self::Builtin(ty) => Cow::Owned(ty.to_string()),
			Self::Primitive(_, name, ..) | Self::Enum { name, .. } => {
				Cow::Borrowed(&ty_env.syms[name.index()])
			}
		}
	}

	#[must_use]
	pub const fn pos(&self) -> Option<Pos> {
		match self {
			Self::Builtin(..) => None,
			Self::Primitive(.., pos) | Self::Enum { pos, .. } => Some(*pos),
		}
	}

	#[must_use]
	pub const fn is_primitive(&self) -> bool {
		matches!(self, Self::Primitive(..))
	}

	#[must_use]
	pub const fn is_int(&self) -> bool {
		matches!(self, Self::Builtin(BuiltinType::Int(..)))
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermKind {
	EnumVariant {
		variant: VariantId,
	},
	Decl {
		flags: TermFlags,
		constructor_kind: Option<ConstructorKind>,
		extractor_kind: Option<ExtractorKind>,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructorKind {
	Internal,
	External { name: Sym },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractorKind {
	Internal {
		template: ast::Pattern,
	},
	External {
		name: Sym,
		infallible: bool,
		pos: Pos,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnKind {
	Plain,
	Option,
	Iterator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
	Bind(TypeId, VarId, Box<Self>),
	Var(TypeId, VarId),
	ConstBool(TypeId, bool),
	ConstInt(TypeId, i128),
	ConstPrim(TypeId, Sym),
	Term(TypeId, TermId, Vec<Self>),
	Wildcard(TypeId),
	And(TypeId, Vec<Self>),
}

impl Pattern {
	#[must_use]
	pub const fn ty(&self) -> TypeId {
		match self {
			Self::Bind(t, ..)
			| Self::Var(t, ..)
			| Self::ConstBool(t, ..)
			| Self::ConstInt(t, ..)
			| Self::ConstPrim(t, ..)
			| Self::Term(t, ..)
			| Self::Wildcard(t, ..)
			| Self::And(t, ..) => *t,
		}
	}

	pub fn visit<V: PatternVisitor>(
		&self,
		visitor: &mut V,
		input: V::PatternId,
		term_env: &TermEnv,
		vars: &mut HashMap<VarId, V::PatternId>,
	) {
		match self {
			Self::Bind(.., var, subpat) => {
				assert!(!vars.contains_key(var));
				vars.insert(*var, input);
				subpat.visit(visitor, input, term_env, vars);
			}
			Self::Var(ty, var) => {
				let var_val = vars
					.get(var)
					.copied()
					.expect("variable should already be bound");
				visitor.add_match_equal(input, var_val, *ty);
			}
			Self::ConstBool(ty, value) => visitor.add_match_bool(input, *ty, *value),
			Self::ConstInt(ty, value) => visitor.add_match_int(input, *ty, *value),
			Self::ConstPrim(ty, value) => visitor.add_match_prim(input, *ty, *value),
			Self::Term(ty, term, args) => {
				let term_data = &term_env.terms[term.index()];
				let arg_values = match term_data.kind {
					TermKind::EnumVariant { variant } => {
						visitor.add_match_variant(input, *ty, &term_data.arg_tys, variant)
					}
					TermKind::Decl {
						extractor_kind: None,
						..
					} => panic!("pattern invocation of undefined term body"),
					TermKind::Decl {
						extractor_kind: Some(ExtractorKind::Internal { .. }),
						..
					} => panic!("should have been expanded away"),
					TermKind::Decl {
						flags,
						extractor_kind: Some(ExtractorKind::External { infallible, .. }),
						..
					} => {
						let output_tys = args.iter().map(Self::ty).collect::<Vec<_>>();

						visitor.add_extract(
							input,
							term_data.ret_ty,
							output_tys,
							*term,
							infallible && !flags.multi,
							flags.multi,
						)
					}
				};

				for (pat, val) in args.iter().zip(arg_values) {
					pat.visit(visitor, val, term_env, vars);
				}
			}
			Self::And(.., children) => {
				children
					.iter()
					.for_each(|child| child.visit(visitor, input, term_env, vars));
			}
			Self::Wildcard(..) => {}
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
	Term(TypeId, TermId, Vec<Self>),
	Var(TypeId, VarId),
	ConstBool(TypeId, bool),
	ConstInt(TypeId, i128),
	ConstPrim(TypeId, Sym),
	Let {
		ty: TypeId,
		bindings: Vec<(VarId, TypeId, Box<Self>)>,
		body: Box<Self>,
	},
}

impl Expr {
	#[must_use]
	pub const fn ty(&self) -> TypeId {
		match self {
			Self::Term(t, ..)
			| Self::Var(t, ..)
			| Self::ConstBool(t, ..)
			| Self::ConstInt(t, ..)
			| Self::ConstPrim(t, ..)
			| Self::Let { ty: t, .. } => *t,
		}
	}

	#[tracing::instrument(level = "trace", skip(visitor, term_env, vars))]
	pub fn visit<V: ExprVisitor>(
		&self,
		visitor: &mut V,
		term_env: &TermEnv,
		vars: &HashMap<VarId, V::ExprId>,
	) -> V::ExprId {
		match self {
			Self::ConstBool(ty, value) => visitor.add_const_bool(*ty, *value),
			Self::ConstInt(ty, value) => visitor.add_const_int(*ty, *value),
			Self::ConstPrim(ty, value) => visitor.add_const_prim(*ty, *value),
			Self::Let { bindings, body, .. } => {
				let mut vars = vars.clone();
				for (var, _, var_expr) in bindings {
					let var_value = var_expr.visit(visitor, term_env, &vars);
					vars.insert(*var, var_value);
				}
				body.visit(visitor, term_env, &vars)
			}
			Self::Var(.., var_id) => *vars.get(var_id).unwrap(),
			Self::Term(ty, term, arg_exprs) => {
				let term_data = &term_env.terms[term.index()];
				let arg_values_tys = arg_exprs
					.iter()
					.map(|arg_expr| arg_expr.visit(visitor, term_env, vars))
					.zip(term_data.arg_tys.iter().copied())
					.collect::<Vec<_>>();

				match term_data.kind {
					TermKind::EnumVariant { variant } => {
						visitor.add_create_variant(arg_values_tys, *ty, variant)
					}
					TermKind::Decl {
						constructor_kind: Some(..),
						flags,
						..
					} => visitor.add_construct(
						arg_values_tys,
						*ty,
						*term,
						flags.pure,
						!flags.partial,
						flags.multi,
					),
					TermKind::Decl {
						constructor_kind: None,
						..
					} => panic!("should have been caught by typechecking"),
				}
			}
		}
	}

	fn visit_in_rule<V: RuleVisitor>(
		&self,
		visitor: &mut V,
		term_env: &TermEnv,
		vars: &HashMap<VarId, <V::PatternVisitor as PatternVisitor>::PatternId>,
	) -> V::Expr {
		let var_exprs = vars
			.iter()
			.map(|(&var, &val)| (var, visitor.pattern_as_expr(val)))
			.collect();
		visitor.add_expr(|visitor| VisitedExpr {
			ty: self.ty(),
			value: self.visit(visitor, term_env, &var_exprs),
		})
	}
}

pub trait PatternVisitor {
	type PatternId: Copy;

	fn add_match_equal(&mut self, a: Self::PatternId, b: Self::PatternId, ty: TypeId);

	fn add_match_bool(&mut self, input: Self::PatternId, ty: TypeId, bool_val: bool);

	fn add_match_int(&mut self, input: Self::PatternId, ty: TypeId, int_val: i128);

	fn add_match_prim(&mut self, input: Self::PatternId, ty: TypeId, val: Sym);

	fn add_match_variant(
		&mut self,
		input: Self::PatternId,
		input_ty: TypeId,
		arg_tys: &[TypeId],
		variant: VariantId,
	) -> Vec<Self::PatternId>;

	fn add_extract(
		&mut self,
		input: Self::PatternId,
		input_ty: TypeId,
		output_tys: impl IntoIterator<Item = TypeId>,
		term: TermId,
		infallible: bool,
		multi: bool,
	) -> Vec<Self::PatternId>;
}

pub trait ExprVisitor {
	type ExprId: Copy;

	fn add_const_bool(&mut self, ty: TypeId, value: bool) -> Self::ExprId;

	fn add_const_int(&mut self, ty: TypeId, value: i128) -> Self::ExprId;

	fn add_const_prim(&mut self, ty: TypeId, value: Sym) -> Self::ExprId;

	fn add_create_variant(
		&mut self,
		inputs: impl IntoIterator<Item = (Self::ExprId, TypeId)>,
		ty: TypeId,
		variant: VariantId,
	) -> Self::ExprId;

	fn add_construct(
		&mut self,
		inputs: impl IntoIterator<Item = (Self::ExprId, TypeId)>,
		ty: TypeId,
		term: TermId,
		pure: bool,
		infallible: bool,
		multi: bool,
	) -> Self::ExprId;
}

pub trait RuleVisitor {
	type PatternVisitor: PatternVisitor;

	type ExprVisitor: ExprVisitor;

	type Expr;

	fn add_arg(
		&mut self,
		index: usize,
		ty: TypeId,
	) -> <Self::PatternVisitor as PatternVisitor>::PatternId;

	fn add_pattern(&mut self, visitor: impl FnOnce(&mut Self::PatternVisitor));

	fn add_expr(
		&mut self,
		visitor: impl FnOnce(&mut Self::ExprVisitor) -> VisitedExpr<Self::ExprVisitor>,
	) -> Self::Expr;

	fn expr_as_pattern(
		&mut self,
		expr: Self::Expr,
	) -> <Self::PatternVisitor as PatternVisitor>::PatternId;

	fn pattern_as_expr(
		&mut self,
		pattern: <Self::PatternVisitor as PatternVisitor>::PatternId,
	) -> <Self::ExprVisitor as ExprVisitor>::ExprId;
}
