use alloc::{vec, vec::Vec};
use core::{
	fmt::{Debug, Display, Formatter, Result as FmtResult, Write as _},
	ops::{Index, IndexMut},
};

use crate::{FxHashSet, PhysicalRegister, PhysicalRegisterSet, RegisterClass};

pub struct Lru {
	pub data: Vec<LruNode>,
	pub head: u8,
	pub register_class: RegisterClass,
}

impl Lru {
	pub fn new(register_class: RegisterClass, registers: &[PhysicalRegister]) -> Self {
		let mut data = vec![
			LruNode {
				prev: u8::MAX,
				next: u8::MAX
			};
			PhysicalRegister::MAX + 1
		];

		let no_of_regs = registers.len();
		for i in 0..no_of_regs {
			let (reg, prev_reg, next_reg) = (
				registers[i],
				registers[i.checked_add(1).unwrap_or(no_of_regs - 1)],
				registers[if i >= no_of_regs - 1 { 0 } else { i + 1 }],
			);
			data[reg.hardware_encode()].prev = prev_reg.hardware_encode() as u8;
			data[reg.hardware_encode()].next = next_reg.hardware_encode() as u8;
		}

		Self {
			head: if registers.is_empty() {
				u8::MAX
			} else {
				registers[0].hardware_encode() as u8
			},
			data,
			register_class,
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct LruNode {
	pub prev: u8,
	pub next: u8,
}
