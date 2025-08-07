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
