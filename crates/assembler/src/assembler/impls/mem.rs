use cranelift_codegen::ir::{InstBuilder as _, MemFlags, Value, types};

use crate::assembler::Assembler;

impl Assembler<'_> {
	pub fn load(&mut self, offset: i32) -> Value {
		if let Some(value) = self.loads.get(&offset) {
			return *value;
		}

		let memory_address = self.memory_address;
		let value = self
			.ins()
			.load(types::I8, Self::memflags(), memory_address, offset);

		self.loads.insert(offset, value);

		value
	}

	pub fn store(&mut self, value: Value, offset: i32) {
		self.invalidate_loads();

		let memory_address = self.memory_address;

		self.ins()
			.store(Self::memflags(), value, memory_address, offset);
	}

	const fn memflags() -> MemFlags {
		MemFlags::trusted()
	}

	pub fn invalidate_loads(&mut self) {
		self.loads.clear();
	}

	pub fn invalidate_load_at(&mut self, offset: i32) {
		self.loads.remove(&offset);
	}
}
