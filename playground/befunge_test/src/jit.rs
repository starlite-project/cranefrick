use std::{fs, mem};

use anyhow::Result;
use cranelift::prelude::*;
use cranelift_codegen::{control::ControlPlane, ir::Inst};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use fxhash::FxHashMap;
use target_lexicon::Triple;

use super::{BinaryOp, CellInt, Instruction, Op, utils};

const CELL_SIZE: i64 = mem::size_of::<CellInt>() as i64;
const CELL_TYPE: Type = if matches!(CELL_SIZE, 4) {
	types::I32
} else {
	types::I64
};

pub fn execute(
	cfg: &[Instruction],
	progbits: &[u8],
	code: &mut [CellInt],
	stack: &mut [CellInt],
	stack_idx: &mut isize,
) -> Result<u32> {
	let mut flag_builder = settings::builder();
	flag_builder.set("use_colocated_libcalls", "false")?;
	flag_builder.set("is_pic", "false")?;
	flag_builder.enable("enable_pcc")?;

	let isa = {
		let builder = isa::lookup(Triple::host())?;
		builder.finish(settings::Flags::new(flag_builder))
	}?;

	let mut func_ctx = FunctionBuilderContext::new();
	let mut jit_builder = JITBuilder::with_isa(isa.clone(), default_libcall_names());

	jit_builder.symbol("pc", utils::put_char as *const u8);
	jit_builder.symbol("pn", utils::put_int as *const u8);
	jit_builder.symbol("gn", utils::read_int as *const u8);
	jit_builder.symbol("r", utils::rand_nibble as *const u8);
	jit_builder.symbol("ps", utils::print_stack as *const u8);

	let mut module = JITModule::new(jit_builder);
	let mut ctx = module.make_context();
	let ptr_type = module.target_config().pointer_type();

	{
		let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

		let mut aligned = MemFlags::new().with_aligned();

        let mut put_sig = module.make_signature();
        put_sig.params.push(AbiParam::new(CELL_TYPE));

            let mut get_sig = module.make_signature();
            get_sig.returns.push(AbiParam::new(CELL_TYPE));

        let put_char = {
            let put_char_fn = module.declare_function("pc", Linkage::Import, &put_sig)?;
            module.declare_func_in_func(put_char_fn, builder.func)
        };

        let get_char = {
            let get_char_fn = module.declare_function("getchar", Linkage::Import, &get_sig)?;
            module.declare_func_in_func(get_char_fn, builder.func)
        };

        let put_num = {
            let put_num_fn = module.declare_function("pn", Linkage::Import, &put_sig)?;

            module.declare_func_in_func(put_num_fn, builder.func)
        };
	}

	fs::write("../../out/unoptimized.clif", ctx.func.to_string())?;

	ctx.optimize(&*isa, &mut ControlPlane::default())?;

	fs::write("../../out/optimized.clif", ctx.func.to_string())?;

	todo!()
}

enum JumpEntry {
	J1(Inst, u32),
	J2(Inst, u32, u32),
}
