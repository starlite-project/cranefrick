use std::collections::{HashMap, HashSet, hash_map::Entry};

use super::{
	error::{Error, Span},
	lexer::Pos,
	sema::{TermEnv, TermFlags, TermId, TermKind},
	trie_again,
};

#[derive(Default)]
struct Errors {
	nodes: HashMap<Pos, HashSet<Pos>>,
	shadowed: HashMap<Pos, Vec<Pos>>,
}

impl Errors {
	fn report(mut self) -> Vec<Error> {
		let mut errors = Vec::new();

		while let Some((&pos, ..)) = self
			.nodes
			.iter()
			.max_by_key(|(pos, edges)| (edges.len(), *pos))
		{
			let node = self.nodes.remove(&pos).unwrap();
			for other in node.iter().copied() {
				if let Entry::Occupied(mut entry) = self.nodes.entry(other) {
					let back_edges = entry.get_mut();
					back_edges.remove(&pos);
					if back_edges.is_empty() {
						entry.remove();
					}
				}
			}

			let mut rules = vec![Span::from(pos)];

			rules.extend(node.into_iter().map(Span::from));

			errors.push(Error::Overlap {
				message: "rules are overlapping".to_owned(),
				rules,
			});
		}

		errors.extend(
			self.shadowed
				.into_iter()
				.map(|(mask, shadowed)| Error::Shadowed {
					shadowed: shadowed.into_iter().map(Span::from).collect(),
					mask: Span::from(mask),
				}),
		);

		errors.sort_by_key(|error| match error {
			Error::Shadowed { mask, .. } => mask.from,
			Error::Overlap { rules, .. } => rules[0].from,
			_ => Pos::default(),
		});

		errors
	}

	fn check_pair(&mut self, a: &trie_again::Rule, b: &trie_again::Rule) {
		if let trie_again::Overlap::Yes { subset } = a.may_overlap(b) {
			if a.prio == b.prio {
				self.nodes.entry(a.pos).or_default().insert(b.pos);
				self.nodes.entry(b.pos).or_default().insert(a.pos);
			} else if subset {
				let (lo, hi) = if a.prio < b.prio { (a, b) } else { (b, a) };
				if hi.total_constraints() <= lo.total_constraints() {
					self.shadowed.entry(hi.pos).or_default().push(lo.pos);
				}
			}
		}
	}
}

pub fn check(term_env: &TermEnv) -> Result<Vec<(TermId, trie_again::RuleSet)>, Vec<Error>> {
	let (terms, mut errors) = trie_again::build(term_env);
	errors.append(&mut check_overlaps(&terms, term_env).report());

	if errors.is_empty() {
		Ok(terms)
	} else {
		Err(errors)
	}
}

fn check_overlaps(terms: &[(TermId, trie_again::RuleSet)], env: &TermEnv) -> Errors {
	let mut errors = Errors::default();

	for (tid, ruleset) in terms {
		let is_multi_ctor = matches!(
			&env.terms[tid.index()].kind,
			TermKind::Decl {
				flags: TermFlags { multi: true, .. },
				..
			}
		);

		if is_multi_ctor {
			continue;
		}

		let mut cursor = ruleset.rules.iter();
		while let Some(left) = cursor.next() {
			for right in cursor.as_slice() {
				errors.check_pair(left, right);
			}
		}
	}

	errors
}
