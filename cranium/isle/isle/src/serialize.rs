use std::{cmp::Reverse, iter, mem};

use super::{
	DisjointSets,
	lexer::Pos,
	trie_again::{Binding, BindingId, Constraint, Rule, RuleSet},
};

#[derive(Debug, Default)]
pub struct Block {
	pub steps: Vec<EvalStep>,
}

#[derive(Debug)]
pub struct EvalStep {
	pub bind_order: Vec<BindingId>,
	pub check: ControlFlow,
}

#[derive(Debug)]
pub struct MatchArm {
	pub constraint: Constraint,
	pub bindings: Vec<Option<BindingId>>,
	pub body: Block,
}

#[derive(Clone, Copy)]
struct PartitionResults {
	any_matched: bool,
	valid: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Score {
	count: usize,
	state: BindingState,
}

impl Score {
	fn update(
		&mut self,
		state: BindingState,
		partition: impl FnOnce() -> PartitionResults,
	) -> bool {
		if matches!(state, BindingState::Matched) {
			return false;
		}

		self.state = state;

		let partition = partition();
		self.count = partition.valid;

		partition.any_matched
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Candidate {
	score: Score,
	kind: Reverse<HasControlFlow>,
}

impl Candidate {
	fn new(kind: HasControlFlow) -> Self {
		Self {
			score: Score::default(),
			kind: Reverse(kind),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct EqualCandidate {
	score: Score,
	source: Reverse<BindingId>,
}

impl EqualCandidate {
	fn new(source: BindingId) -> Self {
		Self {
			score: Score::default(),
			source: Reverse(source),
		}
	}
}

#[derive(Default, Clone)]
struct ScopedState {
	ready: Vec<BindingState>,
	candidates: Vec<Candidate>,
	equal_candidates: Vec<EqualCandidate>,
	equal: DisjointSets<BindingId>,
}

struct Decomposition<'a> {
	rules: &'a RuleSet,
	scope: ScopedState,
	bind_order: Vec<BindingId>,
	block: Block,
}

impl<'a> Decomposition<'a> {
	fn new(rules: &'a RuleSet) -> Self {
		let mut scope = ScopedState::default();
		scope
			.ready
			.resize(rules.bindings.len(), BindingState::default());
		let mut result = Self {
			rules,
			scope,
			bind_order: Vec::new(),
			block: Block::default(),
		};

		result.add_bindings();
		result
	}

	fn new_block(&self) -> Self {
		Self {
			rules: self.rules,
			scope: self.scope.clone(),
			bind_order: Vec::new(),
			block: Block::default(),
		}
	}

	fn add_bindings(&mut self) {
		for (idx, binding) in self.rules.bindings.iter().enumerate() {
			if matches!(
				binding,
				Binding::Iterator { .. } | Binding::MatchVariant { .. } | Binding::MatchSome { .. }
			) {
				continue;
			}

			let idx: BindingId = idx.try_into().unwrap();
			if self.scope.ready[idx.index()] < BindingState::Available
				&& binding
					.sources()
					.iter()
					.all(|&source| self.scope.ready[source.index()] >= BindingState::Available)
			{
				self.set_ready(idx, BindingState::Available);
			}
		}
	}

	fn sort(mut self, mut order: &mut [usize]) -> Block {
		while let Some(best) = self.best_control_flow(order) {
			let partition_point = best.partition(self.rules, order).valid;
			debug_assert!(partition_point > 0);

			let (this, rest) = order.split_at_mut(partition_point);
			order = rest;

			let check = self.make_control_flow(best, this);

			let bind_order = mem::take(&mut self.bind_order);
			self.block.steps.push(EvalStep { bind_order, check });
		}

		debug_assert!(self.scope.candidates.is_empty());

		order.sort_unstable_by_key(|&idx| (Reverse(self.rules.rules[idx].prio), idx));

		for &idx in order.iter() {
			let Rule {
				pos,
				result,
				impure,
				..
			} = &self.rules.rules[idx];

			for &impure in impure {
				self.use_expr(impure);
			}

			self.use_expr(*result);

			let check = ControlFlow::Return {
				pos: *pos,
				result: *result,
			};
			let bind_order = mem::take(&mut self.bind_order);
			self.block.steps.push(EvalStep { bind_order, check });
		}

		self.block
	}

	fn use_expr(&mut self, name: BindingId) {
		if self.scope.ready[name.index()] < BindingState::Emitted {
			self.set_ready(name, BindingState::Emitted);
			let binding = &self.rules.bindings[name.index()];
			for &source in binding.sources() {
				self.use_expr(source);
			}

			let should_let_bind = match binding {
				Binding::ConstInt { .. }
				| Binding::ConstPrim { .. }
				| Binding::Argument { .. }
				| Binding::MatchTuple { .. } => false,
				Binding::MakeVariant { fields, .. } => !fields.is_empty(),
				_ => true,
			};

			if should_let_bind {
				self.bind_order.push(name);
			}
		}
	}

	fn make_control_flow(&mut self, best: HasControlFlow, order: &mut [usize]) -> ControlFlow {
		match best {
			HasControlFlow::Match(source) => {
				self.use_expr(source);
				self.add_bindings();
				let mut arms = Vec::new();

				let get_constraint =
					|idx: usize| self.rules.rules[idx].get_constraints(source).unwrap();

				order.sort_unstable_by_key(|&idx| get_constraint(idx));
				for g in group_by_mut(order, |&a, &b| get_constraint(a) == get_constraint(b)) {
					let mut child = self.new_block();
					child.set_ready(source, BindingState::Matched);

					let constraint = get_constraint(g[0]);
					let bindings = constraint
						.bindings_for(source)
						.into_iter()
						.map(|b| child.rules.find_binding(&b))
						.collect::<Vec<_>>();

					let mut changed = false;
					for &binding in &bindings {
						if let Some(binding) = binding {
							child.set_ready(binding, BindingState::Emitted);
							changed = true;
						}
					}

					if changed {
						child.add_bindings();
					}

					let body = child.sort(g);
					arms.push(MatchArm {
						constraint,
						bindings,
						body,
					});
				}

				ControlFlow::Match { source, arms }
			}
			HasControlFlow::Equal(a, b) => {
				self.use_expr(a);
				self.use_expr(b);
				self.add_bindings();

				let mut child = self.new_block();

				child.scope.equal.merge(a, b);
				let body = child.sort(order);
				ControlFlow::Equal { a, b, body }
			}
			HasControlFlow::Loop(source) => {
				let result = self
					.rules
					.find_binding(&Binding::Iterator { source })
					.unwrap();

				let base_state = self.scope.ready[source.index()];
				debug_assert_eq!(base_state, BindingState::Available);
				self.use_expr(source);
				self.scope.ready[source.index()] = base_state;
				self.add_bindings();

				let mut child = self.new_block();
				child.set_ready(source, BindingState::Matched);
				child.set_ready(result, BindingState::Emitted);
				child.add_bindings();
				let body = child.sort(order);
				ControlFlow::Loop { result, body }
			}
		}
	}

	fn set_ready(&mut self, source: BindingId, state: BindingState) {
		let old = &mut self.scope.ready[source.index()];
		debug_assert!(*old <= state);

		if matches!(old, BindingState::Unavailable) {
			self.scope.candidates.extend([
				Candidate::new(HasControlFlow::Match(source)),
				Candidate::new(HasControlFlow::Loop(source)),
			]);

			self.scope
				.equal_candidates
				.push(EqualCandidate::new(source));
		}

		*old = state;
	}

	fn best_control_flow(&mut self, order: &mut [usize]) -> Option<HasControlFlow> {
		if order.is_empty() {
			self.scope.candidates.clear();
			return None;
		}

		self.scope.candidates.retain_mut(|candidate| {
			let kind = candidate.kind.0;
			let source = match kind {
				HasControlFlow::Match(source) | HasControlFlow::Loop(source) => source,
				HasControlFlow::Equal(..) => unreachable!(),
			};
			let state = self.scope.ready[source.index()];
			candidate
				.score
				.update(state, || kind.partition(self.rules, order))
		});

		let mut best = self.scope.candidates.iter().max().copied();

		self.scope.equal_candidates.retain_mut(|candidate| {
			let source = candidate.source.0;
			let state = self.scope.ready[source.index()];
			candidate.score.update(state, || {
				let matching = partition_in_place(order, |&idx| {
					self.rules.rules[idx].equals.find(source).is_some()
				});

				PartitionResults {
					any_matched: matching > 0,
					valid: respect_priority(self.rules, order, matching),
				}
			})
		});

		self.scope
			.equal_candidates
			.sort_unstable_by(|x, y| y.cmp(x));

		let mut equals = self.scope.equal_candidates.iter();
		while let Some(x) = equals.next() {
			if Some(&x.score) < best.as_ref().map(|best| &best.score) {
				break;
			}

			let x_id = x.source.0;
			for y in equals.as_slice() {
				if Some(&y.score) < best.as_ref().map(|best| &best.score) {
					break;
				}

				let y_id = y.source.0;

				if !self.scope.equal.in_same_set(x_id, y_id) {
					let kind = if x_id < y_id {
						HasControlFlow::Equal(x_id, y_id)
					} else {
						HasControlFlow::Equal(y_id, x_id)
					};
					let pair = Candidate {
						kind: Reverse(kind),
						score: Score {
							count: kind.partition(self.rules, order).valid,
							state: x.score.state.min(y.score.state),
						},
					};

					if best.as_ref() < Some(&pair) {
						best = Some(pair);
					}
				}
			}
		}

		best.filter(|candidate| candidate.score.count > 0)
			.map(|candidate| candidate.kind.0)
	}
}

#[derive(Debug)]
pub enum ControlFlow {
	Match {
		source: BindingId,
		arms: Vec<MatchArm>,
	},
	Equal {
		a: BindingId,
		b: BindingId,
		body: Block,
	},
	Loop {
		result: BindingId,
		body: Block,
	},
	Return {
		pos: Pos,
		result: BindingId,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HasControlFlow {
	Match(BindingId),
	Equal(BindingId, BindingId),
	Loop(BindingId),
}

impl HasControlFlow {
	fn partition(self, rules: &RuleSet, order: &mut [usize]) -> PartitionResults {
		let matching = partition_in_place(order, |&idx| {
			let rule = &rules.rules[idx];
			match self {
				Self::Match(binding_id) => rule.get_constraints(binding_id).is_some(),
				Self::Equal(x, y) => rule.equals.in_same_set(x, y),
				Self::Loop(binding_id) => rule.iterators.contains(&binding_id),
			}
		});

		PartitionResults {
			any_matched: matching > 0,
			valid: respect_priority(rules, order, matching),
		}
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum BindingState {
	#[default]
	Unavailable,
	Available,
	Emitted,
	Matched,
}

#[must_use]
pub fn serialize(rules: &RuleSet) -> Block {
	let mut order = (0..rules.rules.len()).collect::<Vec<_>>();
	Decomposition::new(rules).sort(&mut order)
}

fn respect_priority(rules: &RuleSet, order: &mut [usize], partition_point: usize) -> usize {
	let (selected, deferred) = order.split_at_mut(partition_point);

	if let Some(max_deferred_prio) = deferred.iter().map(|&idx| rules.rules[idx].prio).max() {
		partition_in_place(selected, |&idx| rules.rules[idx].prio >= max_deferred_prio)
	} else {
		partition_point
	}
}

fn partition_in_place<T>(xs: &mut [T], mut pred: impl FnMut(&T) -> bool) -> usize {
	let mut iter = xs.iter_mut();
	let mut partition_point = 0;
	while let Some(a) = iter.next() {
		if pred(a) {
			partition_point += 1;
		} else {
			while let Some(b) = iter.next_back() {
				if pred(b) {
					mem::swap(a, b);
					partition_point += 1;
					break;
				}
			}
		}
	}

	partition_point
}

fn group_by_mut<T: Eq>(
	mut xs: &mut [T],
	mut pred: impl FnMut(&T, &T) -> bool,
) -> impl Iterator<Item = &mut [T]> {
	iter::from_fn(move || {
		if xs.is_empty() {
			None
		} else {
			let mid = xs
				.windows(2)
				.position(|w| !pred(&w[0], &w[1]))
				.map_or(xs.len(), |x| x + 1);
			let slice = mem::take(&mut xs);
			let (group, rest) = slice.split_at_mut(mid);
			xs = rest;
			Some(group)
		}
	})
}

#[cfg(test)]
mod tests {
	use super::group_by_mut;

	#[test]
	fn group_by() {
		let slice = &mut [1, 1, 1, 3, 3, 2, 2, 2];
		let mut iter = group_by_mut(slice, |a, b| a == b);
		assert_eq!(iter.next(), Some(&mut [1, 1, 1][..]));
		assert_eq!(iter.next(), Some(&mut [3, 3][..]));
		assert_eq!(iter.next(), Some(&mut [2, 2, 2][..]));
		assert_eq!(iter.next(), None);
	}
}
