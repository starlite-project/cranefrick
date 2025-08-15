use std::ops::Range;

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

		self.ensure_hint(value, None);

		self.last_value = Some((offset, value));

		value
	}

	pub fn store(&mut self, value: Value, offset: i32, range: Option<Range<u8>>) {
		self.invalidate_load();

		let memory_address = self.memory_address;
		self.ensure_hint(value, range);

		self.ins()
			.store(Self::memflags(), value, memory_address, offset);
	}

	const fn memflags() -> MemFlags {
		MemFlags::trusted()
	}

	pub const fn invalidate_load(&mut self) {
		self.last_value = None;
	}

	pub fn ensure_hint(&mut self, value: Value, range: Option<Range<u8>>) {
		let range = range.map_or(0..(u8::MAX).into(), |r| r.start.into()..r.end.into());

		if self.func.dfg.facts.get(value).is_none() {
			self.func.dfg.facts[value] = Some(Fact::Range {
				bit_width: types::I8.bits() as u16,
				min: range.start,
				max: range.end,
			});
		}
	}
}
