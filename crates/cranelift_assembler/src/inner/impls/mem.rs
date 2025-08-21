use cranelift_codegen::ir::{InstBuilder as _, MemFlags, Value, types};

use crate::inner::InnerAssembler;

impl InnerAssembler<'_> {
	pub fn load(&mut self, offset: i32) -> Value {
		if let Some(value) = self.loads.get(&offset) {
			return *value;
		}

		let ptr_value = self.ptr_value();

		let value = self
			.ins()
			.load(types::I8, Self::memflags(), ptr_value, offset);

		self.loads.insert(offset, value);

		value
	}

	pub fn store(&mut self, value: Value, offset: i32) {
		self.invalidate_load_at(offset);

		let ptr_value = self.ptr_value();

		self.ins().store(Self::memflags(), value, ptr_value, offset);
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

	pub fn invalidate_loads_at(&mut self, offsets: impl IntoIterator<Item = i32>) {
		for offset in offsets {
			self.invalidate_load_at(offset);
		}
	}
}
