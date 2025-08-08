use fxhash::FxHashMap;

use super::CellInt;

#[derive(Debug, Clone)]
pub struct Instruction {
	pub op: Op,
	pub n: u32,
	pub si: Vec<u32>,
	pub depi: Vec<u32>,
	pub depo: u8,
	pub block: bool,
}

impl Instruction {
	#[must_use]
	pub const fn new(op: Op) -> Self {
		let block = matches!(op, Op::Hcf);
		Self {
			op,
			n: 0,
			si: Vec::new(),
			depi: Vec::new(),
			depo: 0,
			block,
		}
	}

	pub fn add_si(&mut self, n: u32, force_block: bool) {
		if !self.si.contains(&n) {
			self.si.push(n);
		}

		if force_block || self.si.len() > 1 {
			self.block = true;
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
	Add,
	Sub,
	Mul,
	Div,
	Mod,
	Cmp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
	Right,
	Up,
	Left,
	Down,
}

impl From<Direction> for u32 {
	fn from(value: Direction) -> Self {
		value as Self
	}
}

impl From<u32> for Direction {
	fn from(value: u32) -> Self {
		match value & 3 {
			0 => Self::Right,
			1 => Self::Up,
			2 => Self::Left,
			_ => Self::Down,
		}
	}
}

#[derive(Debug, Clone)]
pub enum Op {
	Ld(CellInt),
	Binary(BinaryOp),
	Not,
	Pop,
	Dup,
	Swp,
	Rch,
	Wch,
	Rum,
	Wum,
	Rem(Option<u16>),
	Wem(u32, Option<u16>),
	Jr(Box<[u32; 3]>),
	Jz(u32),
	Ret,
	Hcf,
	Nop,
}

impl Op {
	#[must_use]
	pub const fn depth(&self) -> (u8, u8) {
		match self {
			Self::Not => (1, 1),
			Self::Pop | Self::Wch | Self::Wum | Self::Wem(.., Some(..)) | Self::Jz(..) => (1, 0),
			Self::Dup => (1, 2),
			Self::Swp => (2, 2),
			Self::Binary(..) | Self::Rem(None) => (2, 1),
			Self::Ld(..) | Self::Rch | Self::Rum | Self::Rem(Some(..)) => (0, 1),
			Self::Wem(.., None) => (3, 0),
			Self::Jr(..) | Self::Ret | Self::Hcf | Self::Nop => (0, 0),
		}
	}

	#[must_use]
	pub const fn flow(&self) -> bool {
		matches!(self, Self::Jz(..) | Self::Jr(..) | Self::Wem(..))
	}
}

pub fn create_cfg(
	code: &[CellInt],
	pg: &mut [u8; 320],
	xy: u32,
	dir: Direction,
) -> Vec<Instruction> {
	let mut cfg = Vec::new();

	{
		let mut pg_map = FxHashMap::<u32, u32>::default();
		compile(&mut cfg, code, &mut pg_map, xy, dir);
		for &xydir in pg_map.keys() {
			let idx = xydir >> 2;
			pg[idx as usize >> 3] |= 1 << (idx as usize & 7);
		}
	}

	peep(&mut cfg);
	cfg
}

fn emit(
	cfg: &mut Vec<Instruction>,
	prev_inst: &mut u32,
	ret: &mut u32,
	pg_map: &mut FxHashMap<u32, u32>,
	past_spot: &mut Vec<u32>,
	mut instr: Instruction,
) {
	let inst_idx = cfg.len() as u32;
	if matches!(*prev_inst, u32::MAX) {
		*ret = inst_idx;
	} else {
		let prev_op = &mut cfg[*prev_inst as usize];
		prev_op.n = inst_idx;
		if matches!(prev_op.op, Op::Jz(..) | Op::Jr(..)) {
			instr.block = true;
		}

		cfg[*prev_inst as usize].n = inst_idx;
		instr.add_si(*prev_inst, false);
	}

	cfg.push(instr);
	*prev_inst = inst_idx;
	for &spot in past_spot.iter() {
		pg_map.insert(spot, inst_idx);
	}

	past_spot.clear();
}

const fn mv(xy: u32, dir: Direction) -> u32 {
	match dir {
		Direction::Right => {
			if xy >= 2528 {
				xy - 2528
			} else {
				xy + 32
			}
		}
		Direction::Up => {
			if matches!(xy & 31, 0) {
				xy + 24
			} else {
				xy - 1
			}
		}
		Direction::Left => {
			if xy < 32 {
				xy + 2528
			} else {
				xy - 32
			}
		}
		Direction::Down => {
			if ((xy + 1) & 31) < 25 {
				xy + 1
			} else {
				xy - 24
			}
		}
	}
}

fn compile(
	cfg: &mut Vec<Instruction>,
	code: &[CellInt],
	pg_map: &mut FxHashMap<u32, u32>,
	mut xy: u32,
	mut dir: Direction,
) -> u32 {
	let mut tail = u32::MAX;
	let mut head = 0;
	let mut past_spot = Vec::<u32>::new();

	loop {
		let xydir = (xy << 2) | u32::from(dir);
		if past_spot.contains(&xydir) {
			emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Hcf),
			);

			break head;
		}
		past_spot.push(xydir);
		if let Some(&n) = pg_map.get(&xydir) {
			if matches!(tail, u32::MAX) {
				head = n;
			} else {
				cfg[tail as usize].n = n;
				cfg[n as usize].add_si(tail, false);
			}

			for &spot in &past_spot {
				pg_map.insert(spot, n);
			}

			break head;
		}

		let ch = code[xy as usize];
		match ch {
			48..=57 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Ld(ch - 48)),
			),
			58 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Dup),
			),
			92 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Swp),
			),
			36 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Pop),
			),
			103 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Rem(None)),
			),
			112 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Wem(mv(xy, dir) << 2 | u32::from(dir), None)),
			),
			38 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Rum),
			),
			46 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Wum),
			),
			126 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Rch),
			),
			44 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Wch),
			),
			43 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Binary(BinaryOp::Add)),
			),
			45 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Binary(BinaryOp::Sub)),
			),
			42 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Binary(BinaryOp::Mul)),
			),
			47 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Binary(BinaryOp::Div)),
			),
			37 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Binary(BinaryOp::Mod)),
			),
			96 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Binary(BinaryOp::Cmp)),
			),
			33 => emit(
				cfg,
				&mut tail,
				&mut head,
				pg_map,
				&mut past_spot,
				Instruction::new(Op::Not),
			),
			62 => dir = Direction::Right,
			94 => dir = Direction::Up,
			60 => dir = Direction::Left,
			118 => dir = Direction::Down,
			35 => xy = mv(xy, dir),
			95 | 124 => {
				let new_dir = if matches!(ch, 95) {
					(Direction::Right, Direction::Left)
				} else {
					(Direction::Down, Direction::Up)
				};

				emit(
					cfg,
					&mut tail,
					&mut head,
					pg_map,
					&mut past_spot,
					Instruction::new(Op::Nop),
				);
				pg_map.insert(xydir ^ 1, tail);
				pg_map.insert(xydir ^ 2, tail);
				pg_map.insert(xydir ^ 3, tail);
				let d = compile(cfg, code, pg_map, mv(xy, new_dir.0), new_dir.0);
				cfg[tail as usize].op = Op::Jz(d);
				cfg[d as usize].add_si(tail, true);
				dir = new_dir.1;
			}
			63 => {
				emit(
					cfg,
					&mut tail,
					&mut head,
					pg_map,
					&mut past_spot,
					Instruction::new(Op::Nop),
				);
				pg_map.insert(xydir ^ 1, tail);
				pg_map.insert(xydir ^ 2, tail);
				pg_map.insert(xydir ^ 3, tail);
				let d1 = compile(
					cfg,
					code,
					pg_map,
					mv(xy, Direction::Right),
					Direction::Right,
				);
				let d2 = compile(cfg, code, pg_map, mv(xy, Direction::Up), Direction::Up);
				let d3 = compile(cfg, code, pg_map, mv(xy, Direction::Left), Direction::Left);
				cfg[tail as usize].op = Op::Jr(Box::new([d1, d2, d3]));
				cfg[d1 as usize].add_si(tail, true);
				cfg[d2 as usize].add_si(tail, true);
				cfg[d3 as usize].add_si(tail, true);
				dir = Direction::Down;
			}
			64 => {
				emit(
					cfg,
					&mut tail,
					&mut head,
					pg_map,
					&mut past_spot,
					Instruction::new(Op::Ret),
				);
				break head;
			}
			34 => 'inner: loop {
				xy = mv(xy, dir);
				let qch = code[xy as usize];
				if qch == '"' as CellInt {
					break 'inner;
				}

				emit(
					cfg,
					&mut tail,
					&mut head,
					pg_map,
					&mut past_spot,
					Instruction::new(Op::Ld(qch)),
				);
			},
			_ => {}
		}

		xy = mv(xy, dir);
	}
}

fn peep(cfg: &mut [Instruction]) {
	let mut cst = Vec::new();
	let mut idx = 0;
	while (idx as usize) < cfg.len() {
		let (isblock, isflow, (si, mut so)) = {
			let op = &mut cfg[idx as usize];
			(op.block, op.op.flow(), op.op.depth())
		};

		if isblock {
			cst.clear();
		} else {
			for _ in 0..si {
				if let Some(cval) = cst.pop() {
					let c = cval >> 2;
					let cout = cval & 3;
					cfg[c as usize].depo |= 1u8 << cout;
					cfg[idx as usize].depi.push(cval);
				} else {
					break;
				}
			}

			match cfg[idx as usize].op {
				Op::Binary(binop) => {
					if matches!(cfg[idx as usize].depi.len(), 2) {
						let (n0, n1) = {
							let depi = &cfg[idx as usize].depi;
							(depi[0] >> 2, depi[1] >> 2)
						};

						if let Some((v0, v1)) = match (&cfg[n0 as usize].op, &cfg[n1 as usize].op) {
							(&Op::Ld(v0), &Op::Ld(v1)) => Some((v0, v1)),
							_ => None,
						} {
							cfg[n0 as usize].op = Op::Nop;
							cfg[n1 as usize].op = Op::Nop;
							cfg[idx as usize].op = Op::Ld(match binop {
								BinaryOp::Add => v1.wrapping_add(v0),
								BinaryOp::Sub => v1.wrapping_sub(v0),
								BinaryOp::Mul => v1.wrapping_mul(v0),
								BinaryOp::Div => {
									if matches!(v0, 0) {
										0
									} else {
										v1.wrapping_div(v0)
									}
								}
								BinaryOp::Mod => {
									if matches!(v0, 0) {
										0
									} else {
										v1.wrapping_rem(v0)
									}
								}
								BinaryOp::Cmp => CellInt::from(v1 > v0),
							});
							so = 1;
						}
					}
				}
				Op::Rem(None) => {
					if matches!(cfg[idx as usize].depi.len(), 2) {
						let (n0, n1) = {
							let depi = &cfg[idx as usize].depi;
							(depi[0] >> 2, depi[1] >> 2)
						};

						if let Some((v0, v1)) = match (&cfg[n0 as usize].op, &cfg[n1 as usize].op) {
							(&Op::Ld(v0), &Op::Ld(v1)) => Some((v0, v1)),
							_ => None,
						} {
							cfg[n0 as usize].op = Op::Nop;
							cfg[n1 as usize].op = Op::Nop;
							let off = v0 | v1 << 5;
							if (0..2560).contains(&off) {
								cfg[idx as usize].op = Op::Rem(Some(off as u16));
							} else {
								cfg[idx as usize].op = Op::Ld(0);
							}
						}
					}
				}
				Op::Wem(xydir, None) => {
					if cfg[idx as usize].depi.len() >= 2 {
						let (n0, n1) = {
							let depi = &cfg[idx as usize].depi;
							(depi[0] >> 2, depi[1] >> 2)
						};

						if let Some((v0, v1)) = match (&cfg[n0 as usize].op, &cfg[n1 as usize].op) {
							(&Op::Ld(v0), &Op::Ld(v1)) => Some((v0, v1)),
							_ => None,
						} {
							cfg[n0 as usize].op = Op::Nop;
							cfg[n1 as usize].op = Op::Nop;
							cfg[idx as usize].depi.remove(1);
							cfg[idx as usize].depi.remove(0);
							let off = v0 | v1 << 5;
							if (0..2560).contains(&off) {
								cfg[idx as usize].op = Op::Wem(xydir, Some(off as u16));
							} else {
								cfg[idx as usize].op = Op::Pop;
							}
						}
					}
				}
				_ => {}
			}

			if isflow {
				cst.clear();
			}
		}

		for out_idx in 0..so {
			cst.push(idx << 2 | u32::from(out_idx));
		}

		idx += 1;
	}
}
