use std::collections::{HashMap, HashSet};

use koopa::{
	ir::{Function, FunctionData, Value, ValueKind, builder::ValueBuilder as _},
	opt::FunctionPass,
};

pub struct DeadCodeElimination {
	worklist: Vec<Value>,
	liveset: HashSet<Value>,
}

impl DeadCodeElimination {
	#[must_use]
	pub fn new() -> Self {
		Self {
			worklist: Vec::new(),
			liveset: HashSet::new(),
		}
	}

	fn mark(&mut self, data: &FunctionData) {
		for (v, value) in data.dfg().values() {
			if Self::is_critical_inst(value.kind()) {
				self.liveset.insert(*v);
				self.worklist.push(*v);
			}
		}

		while let Some(inst) = self.worklist.pop() {
			for u in data.dfg().value(inst).kind().value_uses() {
				if !self.liveset.contains(&u)
					&& data
						.dfg()
						.values()
						.get(&u)
						.is_some_and(|v| v.kind().is_local_inst())
				{
					self.liveset.insert(u);
					self.worklist.push(u);
				}
			}
		}
	}

	fn sweep(&self, data: &mut FunctionData) {
		let mut removed = Vec::new();
		let mut bb_cur = data.layout_mut().bbs_mut().cursor_front_mut();
		while let Some(bb) = bb_cur.node_mut() {
			let mut inst_cur = bb.insts_mut().cursor_front_mut();
			while let Some(inst) = inst_cur.key() {
				if self.liveset.contains(inst) {
					inst_cur.move_next();
				} else {
					removed.push(*inst);
					inst_cur.remove_current();
				}
			}

			bb_cur.move_next();
		}

		for v in removed {
			data.dfg_mut().remove_value(v);
		}
	}

	fn opt_bb_params(&self, data: &mut FunctionData) -> bool {
		let bbs: HashMap<_, _> = data
			.dfg()
			.bbs()
			.iter()
			.filter_map(|(b, bb)| {
				let unused: HashMap<_, _> = bb
					.params()
					.iter()
					.enumerate()
					.filter_map(|(i, p)| {
						data.dfg().value(*p).used_by().is_empty().then_some((i, *p))
					})
					.collect();

				(!unused.is_empty()).then_some((*b, unused))
			})
			.collect();

		let changed = !bbs.is_empty();

		bbs.iter().for_each(|(b, m)| {
			let mut index = 0;
			data.dfg_mut().bb_mut(*b).params_mut().retain(|_| {
				index += 1;
				!m.contains_key(&(index - 1))
			});

			m.iter().for_each(|(_, param)| {
				data.dfg_mut().remove_value(*param);
			});
		});

		bbs.into_iter().for_each(|(b, m)| {
			let users = data.dfg().bb(b).used_by().clone();
			users.into_iter().for_each(|user| {
				let mut inst = data.dfg().value(user).clone();
				let args = match inst.kind_mut() {
					ValueKind::Branch(br) => {
						if br.true_bb() == b {
							br.true_args_mut()
						} else {
							br.false_args_mut()
						}
					}
					ValueKind::Jump(jump) => jump.args_mut(),
					_ => panic!("invalid branch/jump instruction"),
				};

				let mut removed_args = HashSet::new();
				let mut index = 0;
				args.retain(|a| {
					index += 1;

					let removed = !m.contains_key(&(index - 1));
					if removed {
						removed_args.insert(*a);
					}
					removed
				});

				data.dfg_mut().replace_value_with(user).raw(inst);

				removed_args.into_iter().for_each(|v| {
					if data.dfg().value(v).used_by().is_empty() {
						data.dfg_mut().remove_value(v);
					}
				});
			});
		});

		changed
	}

	const fn is_critical_inst(kind: &ValueKind) -> bool {
		matches!(
			kind,
			ValueKind::Store(..)
				| ValueKind::Call(..)
				| ValueKind::Branch(..)
				| ValueKind::Jump(..)
				| ValueKind::Return(..)
		)
	}
}

impl Default for DeadCodeElimination {
	fn default() -> Self {
		Self::new()
	}
}

impl FunctionPass for DeadCodeElimination {
	fn run_on(&mut self, _: Function, data: &mut FunctionData) {
		let mut changed = true;

		while changed {
			self.mark(data);
			self.sweep(data);

			changed = self.opt_bb_params(data);
		}
	}
}
