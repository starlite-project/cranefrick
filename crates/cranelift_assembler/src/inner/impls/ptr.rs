use cranelift_codegen::ir::InstBuilder as _;

use crate::inner::{InnerAssembler, SrcLoc};

impl InnerAssembler<'_> {
	pub fn move_pointer(&mut self, offset: i32) {
		self.shift_load_offsets(offset);

		self.add_srcflag(SrcLoc::MOVE_POINTER);

		let ptr_type = self.ptr_type;
		let ptr_var = self.ptr_variable;
		let ptr_value = self.ptr_value();

		let value = self.ins().iconst(ptr_type, i64::from(offset));
		let new_ptr_value = self.ins().iadd(ptr_value, value);

		self.def_var(ptr_var, new_ptr_value);

		self.remove_srcflag(SrcLoc::MOVE_POINTER);
	}

	fn shift_load_offsets(&mut self, offset: i32) {
		let loads = self.loads.clone();

		self.invalidate_loads();

		for (key, value) in loads {
			self.loads.insert(key.wrapping_sub(offset), value);
		}
	}
}
