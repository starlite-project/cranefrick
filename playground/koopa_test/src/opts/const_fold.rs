use koopa::{
	ir::{BinaryOp, Function, FunctionData, Type, ValueKind, builder::ValueBuilder},
	opt::FunctionPass,
};

pub struct ConstantFolding;

impl ConstantFolding {
	#[must_use]
	pub const fn new() -> Self {
		Self
	}

	fn eval_const(&self, data: &mut FunctionData) -> bool {
		let mut evaluated = Vec::new();

		for (v, value) in data.dfg().values() {
			let ans = match value.kind() {
				ValueKind::Binary(bin) => {
					let lhs = data.dfg().value(bin.lhs()).kind();
					let rhs = data.dfg().value(bin.rhs()).kind();
					match (lhs, rhs) {
						(ValueKind::Integer(l), ValueKind::Integer(r)) => match bin.op() {
							BinaryOp::NotEq => Some(i32::from(l.value() != r.value())),
							BinaryOp::Eq => Some(i32::from(l.value() == r.value())),
							BinaryOp::Gt => Some(i32::from(l.value() > r.value())),
							BinaryOp::Ge => Some(i32::from(l.value() >= r.value())),
							BinaryOp::Lt => Some(i32::from(l.value() < r.value())),
							BinaryOp::Le => Some(i32::from(l.value() <= r.value())),
							BinaryOp::Add => Some(l.value() + r.value()),
							BinaryOp::Sub => Some(l.value() - r.value()),
							BinaryOp::Mul => Some(l.value() * r.value()),
							BinaryOp::Div => (r.value() != 0).then(|| l.value() / r.value()),
							BinaryOp::Mod => (r.value() != 0).then(|| l.value() % r.value()),
							BinaryOp::And => Some(l.value() & r.value()),
							BinaryOp::Or => Some(l.value() | r.value()),
							BinaryOp::Xor => Some(l.value() ^ r.value()),
							BinaryOp::Shl => Some(l.value() << r.value()),
							BinaryOp::Shr => Some((l.value() as u32 >> r.value()) as i32),
							BinaryOp::Sar => Some(l.value() >> r.value()),
						},
						_ => continue,
					}
				}
				_ => continue,
			};

			evaluated.push((*v, ans, data.layout().parent_bb(*v).unwrap()));
		}

		let changed = !evaluated.is_empty();

		for (v, ans, _) in &evaluated {
			let builder = data.dfg_mut().replace_value_with(*v);
			if let Some(v) = ans {
				builder.integer(*v);
			} else {
				builder.undef(Type::get_i32());
			}
		}

		for (v, _, bb) in evaluated {
			data.layout_mut().bb_mut(bb).insts_mut().remove(&v);
		}

		changed
	}

	fn eval_bb_params(&self, data: &mut FunctionData) {
		let mut bb_params = Vec::new();
		for (b, bb) in data.dfg().bbs() {
			let mut evaluated = Vec::new();
			'outer: for i in 0..bb.params().len() {
				let mut ans = None;
				for user in bb.used_by() {
					let value = match data.dfg().value(*user).kind() {
						ValueKind::Branch(branch) => {
							if branch.true_bb() == *b {
								branch.true_args()[i]
							} else {
								branch.false_args()[i]
							}
						}
						ValueKind::Jump(jump) => jump.args()[i],
						_ => panic!("invalid branch/jump instruction"),
					};

					let value = data.dfg().value(value);
					if !value.kind().is_const()
						|| !ans.is_none_or(|v| data.dfg().data_eq(&v, value))
					{
						continue 'outer;
					}

					ans = Some(value.clone());
				}

				evaluated.push((i, ans.unwrap()));
			}

			if !evaluated.is_empty() {
				bb_params.push((*b, evaluated));
			}
		}

		for (bb, evals) in bb_params {
			for (i, value) in evals {
				let p = data.dfg().bb(bb).params()[i];
				let param = data.dfg().value(p).clone();
				data.dfg_mut().replace_value_with(p).raw(value);
				let p = data.dfg_mut().new_value().raw(param);
				data.dfg_mut().bb_mut(bb).params_mut()[i] = p;
			}
		}
	}
}

impl Default for ConstantFolding {
	fn default() -> Self {
		Self::new()
	}
}

impl FunctionPass for ConstantFolding {
	fn run_on(&mut self, _: Function, data: &mut FunctionData) {
		while self.eval_const(data) {}
		self.eval_bb_params(data);
	}
}
