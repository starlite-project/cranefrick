use cranelift_codegen::ir::{InstBuilder as _, MemFlags, Value, types};

use crate::inner::InnerAssembler;

#[expect(unused)]
impl InnerAssembler<'_> {
	pub fn load(&mut self, offset: i32) -> Value {
		let (loaded_value, ..) = self.load_from(offset);

		loaded_value
	}

	pub fn load_from(&mut self, offset: i32) -> (Value, Value) {
		let tape_slot = self.tape;
		let ptr_type = self.ptr_type;

		let tape_addr = self.ins().stack_addr(ptr_type, tape_slot, 0);

		let pointer_value = self.offset_pointer(offset);

		let pointer_offset_tape = self.ins().iadd(tape_addr, pointer_value);

		let tape_value = self
			.ins()
			.load(types::I8, Self::memflags(), pointer_offset_tape, 0);

		(tape_value, pointer_offset_tape)
	}

	pub fn store(&mut self, value: Value, offset: i32) {
		let tape_slot = self.tape;
		let ptr_type = self.ptr_type;

		let tape_addr = self.ins().stack_addr(ptr_type, tape_slot, 0);

		let pointer_value = self.offset_pointer(offset);

		let pointer = self.ins().iadd(tape_addr, pointer_value);

		self.store_into(value, pointer);
	}

	pub fn store_into(&mut self, value: Value, pointer: Value) {
		self.ins().store(Self::memflags(), value, pointer, 0);
	}

	pub fn store_value(&mut self, value: u8, offset: i32) {
		let value = self.ins().iconst(types::I8, i64::from(value));

		self.store(value, offset);
	}

	pub fn store_value_into(&mut self, value: u8, pointer: Value) {
		let value = self.ins().iconst(types::I8, i64::from(value));

		self.store_into(value, pointer);
	}

	pub fn take(&mut self, offset: i32) -> Value {
		let (value, pointer) = self.load_from(offset);

		self.store_value_into(0, pointer);

		value
	}

	const fn memflags() -> MemFlags {
		MemFlags::new()
	}
}
