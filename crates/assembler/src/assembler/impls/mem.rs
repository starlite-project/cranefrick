use std::ops::Range;

use cranelift_codegen::ir::{Fact, InstBuilder as _, MemFlags, Value, types};

use crate::assembler::Assembler;

impl Assembler<'_> {
	pub fn load(&mut self, offset: i32) -> Value {
		// let memory_address = self.memory_address;
		// let value = self
		// 	.ins()
		// 	.load(types::I8, Self::memflags(), memory_address, offset);

		// value

		let offset = if offset.is_positive() {
			(self.tape_idx + offset as usize) % 30_000
		} else {
			(self.tape_idx + 30_000 - offset as usize) % 30_000
		};

		self.tape[offset]
	}

	pub fn store(&mut self, value: Value, offset: i32) {
		// let memory_address = self.memory_address;

		// self.ins()
		// 	.store(Self::memflags(), value, memory_address, offset);

		let offset = if offset.is_positive() {
			(self.tape_idx + offset as usize) % 30_000
		} else {
			(self.tape_idx + 30_000 - offset as usize) % 30_000
		};

		self.tape[offset] = value;
	}

	const fn memflags() -> MemFlags {
		MemFlags::trusted()
	}
}
