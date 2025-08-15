use cranelift_codegen::ir::InstBuilder as _;

use crate::assembler::Assembler;

impl Assembler<'_> {
    pub fn move_pointer(&mut self, offset: i32) {
        // self.invalidate_loads();
        self.shift_load_offsets(offset);

        let ptr_type = self.ptr_type;
        let memory_address = self.memory_address;

        let value =self.ins().iconst(ptr_type, i64::from(offset));
        self.memory_address = self.ins().iadd(memory_address, value);
    }

    fn shift_load_offsets(&mut self, offset: i32) {
        let loads = self.loads.clone();

        self.invalidate_loads();

        for (key, value) in loads {
            self.loads.insert(key.wrapping_add(offset), value);
        }
    }
}
