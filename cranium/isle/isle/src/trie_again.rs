use std::{
	collections::{HashMap, hash_map::Entry},
	mem, slice,
};

use cranefrick_utils::IntoIteratorExt as _;

use super::{
	DisjointSets, StableSet,
	error::{Error, Span},
	lexer::Pos,
	sema,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TupleIndex(u8);

impl TupleIndex {
	#[must_use]
	pub const fn index(self) -> usize {
		self.0 as usize
	}
}

impl TryFrom<usize> for TupleIndex {
	type Error = <u8 as TryFrom<usize>>::Error;

	fn try_from(value: usize) -> Result<Self, Self::Error> {
		value.try_into().map(Self)
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BindingId(u16);

impl BindingId {
	#[must_use]
	pub const fn index(self) -> usize {
		self.0 as usize
	}
}

impl TryFrom<usize> for BindingId {
	type Error = <u16 as TryFrom<usize>>::Error;

	fn try_from(value: usize) -> Result<Self, Self::Error> {
		value.try_into().map(Self)
	}
}

#[derive(Debug, Default)]
pub struct Rule {
	pub pos: Pos,
	constraints: HashMap<BindingId, Constraint>,
	pub equals: DisjointSets<BindingId>,
	pub iterators: StableSet<BindingId>,
	pub prio: i64,
	pub impure: Vec<BindingId>,
	pub result: BindingId,
}

impl Rule {
	#[must_use]
	pub fn may_overlap(&self, other: &Self) -> Overlap {
		let (small, big) = if self.constraints.len() <= other.constraints.len() {
			(self, other)
		} else {
			(other, self)
		};

		let mut subset = small.equals.is_empty() && big.equals.is_empty();

		for (binding, a) in &small.constraints {
			if let Some(b) = big.constraints.get(binding) {
				if a != b {
					return Overlap::No;
				}
			} else {
				subset = false;
			}
		}

		Overlap::Yes { subset }
	}

	#[must_use]
	pub fn total_constraints(&self) -> usize {
		self.constraints.len() + self.equals.len()
	}

	#[must_use]
	pub fn get_constraints(&self, source: BindingId) -> Option<Constraint> {
		self.constraints.get(&source).copied()
	}

	fn set_constraint(
		&mut self,
		source: BindingId,
		constraint: Constraint,
	) -> Result<(), UnreachableError> {
		match self.constraints.entry(source) {
			Entry::Occupied(entry) => {
				if entry.get() != &constraint {
					return Err(UnreachableError {
						pos: self.pos,
						constraint_a: *entry.get(),
						constraint_b: constraint,
					});
				}
			}
			Entry::Vacant(entry) => {
				entry.insert(constraint);
			}
		}

		Ok(())
	}
}

#[derive(Debug, Default)]
pub struct RuleSet {
	pub rules: Vec<Rule>,
	pub bindings: Vec<Binding>,
	binding_map: HashMap<Binding, BindingId>,
}

impl RuleSet {
	#[must_use]
	pub fn find_binding(&self, binding: &Binding) -> Option<BindingId> {
		self.binding_map.get(binding).copied()
	}
}

#[derive(Debug, Clone, Copy)]
struct UnreachableError {
	pos: Pos,
	constraint_a: Constraint,
	constraint_b: Constraint,
}

#[derive(Debug, Default)]
struct RuleSetBuilder {
	current_rule: Rule,
	impure_instance: u32,
	unreachable: Vec<UnreachableError>,
	rules: RuleSet,
}

impl RuleSetBuilder {
	fn add_rule(&mut self, rule: &sema::Rule, term_env: &sema::TermEnv, errors: &mut Vec<Error>) {
		self.impure_instance = 0;
		self.current_rule.pos = rule.pos;
		self.current_rule.prio = rule.prio;
		self.current_rule.result = rule.visit(self, term_env);
		if term_env.terms[rule.root_term.index()].is_partial() {
			self.current_rule.result = self.dedup_binding(Binding::MakeSome {
				inner: self.current_rule.result,
			});
		}

		self.normalize_equivalence_classes();

		let rule = mem::take(&mut self.current_rule);

		if self.unreachable.is_empty() {
			self.rules.rules.push(rule);
		} else {
			errors.extend(self.unreachable.drain(..).map(|error| Error::Unreachable {
				message: format!(
					"rule requires binding to match both {:?} and {:?}",
					error.constraint_a, error.constraint_b
				),
				span: Span::from_single(error.pos),
			}));
		}
	}

	fn normalize_equivalence_classes(&mut self) {
		let mut deferred_constraints = Vec::new();
		for (&binding, &constraint) in &self.current_rule.constraints {
			if let Some(root) = self.current_rule.equals.find_mut(binding) {
				deferred_constraints.push((root, constraint));
			}
		}

		while let Some((current, constraint)) = deferred_constraints.pop() {
			let set = self.current_rule.equals.remove_set_of(current);
			if let Some((&base, rest)) = set.split_first() {
				let mut defer = |this: &Self, binding| {
					if let Some(constraint) = this.current_rule.get_constraints(binding) {
						deferred_constraints.push((binding, constraint));
					}
				};

				let base_fields = self.set_constraint(base, constraint);
				base_fields.iter().for_each(|&x| defer(self, x));
				for &b in rest {
					for (&x, y) in base_fields.iter().zip(self.set_constraint(b, constraint)) {
						defer(self, y);
						self.current_rule.equals.merge(x, y);
					}
				}
			}
		}
	}

	fn dedup_binding(&mut self, binding: Binding) -> BindingId {
		if let Some(binding) = self.rules.binding_map.get(&binding) {
			*binding
		} else {
			let id = self.rules.bindings.len().try_into().unwrap();
			self.rules.bindings.push(binding.clone());
			self.rules.binding_map.insert(binding, id);
			id
		}
	}

	fn set_constraint(&mut self, input: BindingId, constraint: Constraint) -> Vec<BindingId> {
		if let Err(e) = self.current_rule.set_constraint(input, constraint) {
			self.unreachable.push(e);
		}

		constraint
			.bindings_for(input)
			.into_iter()
			.map(|binding| self.dedup_binding(binding))
			.collect()
	}
}

impl sema::ExprVisitor for RuleSetBuilder {
	type ExprId = BindingId;

	fn add_const_bool(&mut self, ty: sema::TypeId, value: bool) -> Self::ExprId {
		self.dedup_binding(Binding::ConstBool { value, ty })
	}

	fn add_const_int(&mut self, ty: sema::TypeId, value: i128) -> Self::ExprId {
		self.dedup_binding(Binding::ConstInt { value, ty })
	}

	fn add_const_prim(&mut self, _: sema::TypeId, value: sema::Sym) -> Self::ExprId {
		self.dedup_binding(Binding::ConstPrim { value })
	}

	fn add_create_variant(
		&mut self,
		inputs: impl IntoIterator<Item = (Self::ExprId, sema::TypeId)>,
		ty: sema::TypeId,
		variant: sema::VariantId,
	) -> Self::ExprId {
		self.dedup_binding(Binding::MakeVariant {
			ty,
			variant,
			fields: inputs.into_iter().map(|(expr, ..)| expr).collect(),
		})
	}

	fn add_construct(
		&mut self,
		inputs: impl IntoIterator<Item = (Self::ExprId, sema::TypeId)>,
		_: sema::TypeId,
		term: sema::TermId,
		pure: bool,
		infallible: bool,
		multi: bool,
	) -> Self::ExprId {
		let instance = if pure {
			0
		} else {
			self.impure_instance += 1;
			self.impure_instance
		};

		let source = self.dedup_binding(Binding::Constructor {
			term,
			parameters: inputs.into_iter().map(|(expr, ..)| expr).collect(),
			instance,
		});

		let source = if multi {
			self.current_rule.iterators.insert(source);
			self.dedup_binding(Binding::Iterator { source })
		} else if infallible {
			source
		} else {
			self.dedup_binding(Binding::MatchSome { source })
		};

		if !pure {
			self.current_rule.impure.push(source);
		}

		source
	}
}

impl sema::PatternVisitor for RuleSetBuilder {
	type PatternId = BindingId;

	fn add_match_equal(&mut self, a: Self::PatternId, b: Self::PatternId, _: sema::TypeId) {
		if a != b {
			self.current_rule.equals.merge(a, b);
		}
	}

	fn add_match_bool(&mut self, input: Self::PatternId, ty: sema::TypeId, bool_val: bool) {
		let bindings = self.set_constraint(
			input,
			Constraint::ConstBool {
				value: bool_val,
				ty,
			},
		);
		debug_assert!(bindings.is_empty());
	}

	fn add_match_int(&mut self, input: Self::PatternId, ty: sema::TypeId, int_val: i128) {
		let bindings = self.set_constraint(input, Constraint::ConstInt { value: int_val, ty });
		debug_assert!(bindings.is_empty());
	}

	fn add_match_prim(&mut self, input: Self::PatternId, _: sema::TypeId, val: sema::Sym) {
		let bindings = self.set_constraint(input, Constraint::ConstPrim { value: val });
		debug_assert!(bindings.is_empty());
	}

	fn add_match_variant(
		&mut self,
		input: Self::PatternId,
		input_ty: sema::TypeId,
		arg_tys: &[sema::TypeId],
		variant: sema::VariantId,
	) -> Vec<Self::PatternId> {
		let fields = TupleIndex(arg_tys.len().try_into().unwrap());
		self.set_constraint(
			input,
			Constraint::Variant {
				ty: input_ty,
				variant,
				fields,
			},
		)
	}

	fn add_extract(
		&mut self,
		input: Self::PatternId,
		_: sema::TypeId,
		output_tys: impl IntoIterator<Item = sema::TypeId>,
		term: sema::TermId,
		infallible: bool,
		multi: bool,
	) -> Vec<Self::PatternId> {
		let source = self.dedup_binding(Binding::Extractor {
			term,
			parameter: input,
		});

		let source = if multi {
			self.current_rule.iterators.insert(source);
			self.dedup_binding(Binding::Iterator { source })
		} else if infallible {
			source
		} else {
			let bindings = self.set_constraint(source, Constraint::Some);
			debug_assert_eq!(bindings.len(), 1);
			bindings[0]
		};

		let output_tys = output_tys.collect_to::<Vec<_>>();

		match output_tys.len().try_into().unwrap() {
			0 => Vec::new(),
			1 => vec![source],
			outputs => (0..outputs)
				.map(TupleIndex)
				.map(|field| self.dedup_binding(Binding::MatchTuple { source, field }))
				.collect(),
		}
	}
}

impl sema::RuleVisitor for RuleSetBuilder {
	type Expr = BindingId;
	type ExprVisitor = Self;
	type PatternVisitor = Self;

	fn add_arg(
		&mut self,
		index: usize,
		_: sema::TypeId,
	) -> <Self::PatternVisitor as sema::PatternVisitor>::PatternId {
		let index: TupleIndex = index.try_into().unwrap();
		self.dedup_binding(Binding::Argument { index })
	}

	fn add_pattern(&mut self, visitor: impl FnOnce(&mut Self::PatternVisitor)) {
		visitor(self);
	}

	fn add_expr(
		&mut self,
		visitor: impl FnOnce(&mut Self::ExprVisitor) -> sema::VisitedExpr<Self::ExprVisitor>,
	) -> Self::Expr {
		visitor(self).value
	}

	fn expr_as_pattern(
		&mut self,
		expr: Self::Expr,
	) -> <Self::PatternVisitor as sema::PatternVisitor>::PatternId {
		let mut todo = vec![expr];
		while let Some(expr) = todo.pop() {
			let expr = &self.rules.bindings[expr.index()];
			todo.extend_from_slice(expr.sources());
			if let Binding::MatchSome { source } = expr {
				_ = self.set_constraint(*source, Constraint::Some);
			}
		}

		expr
	}

	fn pattern_as_expr(
		&mut self,
		pattern: <Self::PatternVisitor as sema::PatternVisitor>::PatternId,
	) -> <Self::ExprVisitor as sema::ExprVisitor>::ExprId {
		pattern
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Binding {
	ConstBool {
		value: bool,
		ty: sema::TypeId,
	},
	ConstInt {
		value: i128,
		ty: sema::TypeId,
	},
	ConstPrim {
		value: sema::Sym,
	},
	Argument {
		index: TupleIndex,
	},
	Extractor {
		term: sema::TermId,
		parameter: BindingId,
	},
	Constructor {
		term: sema::TermId,
		parameters: Box<[BindingId]>,
		instance: u32,
	},
	Iterator {
		source: BindingId,
	},
	MakeVariant {
		ty: sema::TypeId,
		variant: sema::VariantId,
		fields: Box<[BindingId]>,
	},
	MatchVariant {
		source: BindingId,
		variant: sema::VariantId,
		field: TupleIndex,
	},
	MakeSome {
		inner: BindingId,
	},
	MatchSome {
		source: BindingId,
	},
	MatchTuple {
		source: BindingId,
		field: TupleIndex,
	},
}

impl Binding {
	#[must_use]
	pub fn sources(&self) -> &[BindingId] {
		match self {
			Self::Extractor { parameter, .. } => slice::from_ref(parameter),
			Self::Constructor { parameters, .. } => &parameters[..],
			Self::Iterator { source }
			| Self::MatchVariant { source, .. }
			| Self::MatchSome { source }
			| Self::MatchTuple { source, .. } => slice::from_ref(source),
			Self::MakeVariant { fields, .. } => &fields[..],
			Self::MakeSome { inner } => slice::from_ref(inner),
			_ => &[][..],
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Constraint {
	Variant {
		ty: sema::TypeId,
		variant: sema::VariantId,
		fields: TupleIndex,
	},
	ConstBool {
		value: bool,
		ty: sema::TypeId,
	},
	ConstInt {
		value: i128,
		ty: sema::TypeId,
	},
	ConstPrim {
		value: sema::Sym,
	},
	Some,
}

impl Constraint {
	pub fn bindings_for(self, source: BindingId) -> Vec<Binding> {
		match self {
			Self::Some => vec![Binding::MatchSome { source }],
			Self::Variant {
				variant, fields, ..
			} => (0..fields.0)
				.map(TupleIndex)
				.map(|field| Binding::MatchVariant {
					source,
					variant,
					field,
				})
				.collect(),
			_ => Vec::new(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlap {
	No,
	Yes { subset: bool },
}

pub fn build(term_env: &sema::TermEnv) -> (Vec<(sema::TermId, RuleSet)>, Vec<Error>) {
	let mut errors = Vec::new();
	let mut term = HashMap::new();
	for rule in &term_env.rules {
		term.entry(rule.root_term)
			.or_insert_with(RuleSetBuilder::default)
			.add_rule(rule, term_env, &mut errors);
	}

	let mut results = term
		.into_iter()
		.map(|(term, builder)| (term, builder.rules))
		.collect::<Vec<_>>();
	results.sort_unstable_by_key(|(term, ..)| *term);

	(results, errors)
}
