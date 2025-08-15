use cranelift_codegen::ir::{Fact, InstBuilder as _, MemFlags, Value, types};

use crate::assembler::Assembler;

impl Assembler<'_> {
	pub fn load(&mut self, offset: i32) -> Value {
		if let Some((last_offset, value)) = self.last_value
			&& last_offset == offset
		{
			return value;
		}

		let memory_address = self.memory_address;
		let value = self
			.ins()
			.load(types::I8, Self::memflags(), memory_address, offset);

		self.ensure_hint(value);

		self.last_value = Some((offset, value));

		value
	}

	pub fn store(&mut self, value: Value, offset: i32) {
		self.invalidate_load();

		let memory_address = self.memory_address;
		self.ensure_hint(value);

		self.ins()
			.store(Self::memflags(), value, memory_address, offset);
	}

	const fn memflags() -> MemFlags {
		MemFlags::trusted()
	}

	pub const fn invalidate_load(&mut self) {
		self.last_value = None;
	}

	fn ensure_hint(&mut self, value: Value) {
		if self.func.dfg.facts.get(value).is_none() {
			self.func.dfg.facts[value] = Some(Fact::Range {
				bit_width: types::I8.bits() as u16,
				min: 0,
				max: u8::MAX.into(),
			});
		}
	}
}
