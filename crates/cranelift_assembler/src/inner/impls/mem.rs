use cranelift_codegen::ir::{InstBuilder as _, MemFlags, Value, types};

use crate::inner::{InnerAssembler, SrcLoc};

impl InnerAssembler<'_> {
	pub fn load(&mut self, offset: i32) -> Value {
		// if let Some(value) = self.loads.get(&offset) {
		// 	return *value;
		// }

		// let ptr_value = self.ptr_value();

		// let value = self
		// 	.ins()
		// 	.load(types::I8, Self::memflags(), ptr_value, offset);

		// self.loads.insert(offset, value);

		// value

		let offset = self.calculate_ptr(offset);

		let var = self.cells.get(&offset).copied().unwrap();

		self.use_var(var)
	}

	pub fn store(&mut self, value: Value, offset: i32) {
		// self.invalidate_load_at(offset);

		// let ptr_value = self.ptr_value();

		// self.ins().store(Self::memflags(), value, ptr_value, offset);

		let offset = self.calculate_ptr(offset);

		let var = self.cells.get(&offset).copied().unwrap();

		self.def_var(var, value);
	}

	const fn memflags() -> MemFlags {
		MemFlags::trusted()
	}
}
