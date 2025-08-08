use std::{fs, mem};

use color_eyre::{Result, eyre::ContextCompat};
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
	code: &[CellInt],
	stack: &[CellInt],
	stack_idx: &isize,
	iteration: usize,
) -> Result<u32> {
	let mut flag_builder = settings::builder();
	flag_builder.set("use_colocated_libcalls", "false")?;
	flag_builder.set("is_pic", "false")?;
	flag_builder.enable("enable_pcc")?;
	flag_builder.set("opt_level", "speed_and_size")?;

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

	ctx.func.signature.returns.push(AbiParam::new(types::I32));

	{
		let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

		let aligned = MemFlags::new().with_aligned();

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

		let get_num = {
			let get_num_fn = module.declare_function("gn", Linkage::Import, &get_sig)?;

			module.declare_func_in_func(get_num_fn, builder.func)
		};

		let rand = {
			let mut rand_sig = module.make_signature();
			rand_sig.returns.push(AbiParam::new(types::I8));

			let rand_fn = module.declare_function("r", Linkage::Import, &rand_sig)?;

			module.declare_func_in_func(rand_fn, builder.func)
		};

		let ps = {
			let mut ps_sig = module.make_signature();
			ps_sig.params.extend([AbiParam::new(ptr_type); 2]);

			let ps_fn = module.declare_function("ps", Linkage::Import, &ps_sig)?;

			module.declare_func_in_func(ps_fn, builder.func)
		};

		let mut block_filled = false;
		let entry_block = builder.create_block();
		builder.switch_to_block(entry_block);

		let zero = builder.ins().iconst(CELL_TYPE, 0);
		let zero_i8 = builder.ins().iconst(types::I8, 0);
		let one_i8 = builder.ins().iconst(types::I8, 1);
		let negative_one_i32 = builder.ins().iconst(types::I32, -1);
		let two_i8 = builder.ins().iconst(types::I8, 2);
		let two_five_six_zero_i32 = builder.ins().iconst(types::I32, 2560);
		let null = builder.ins().iconst(ptr_type, 0);
		let stack_idx_const = builder.ins().iconst(ptr_type, (*stack_idx * 4) as i64);
		let vstack = builder.ins().iconst(ptr_type, stack.as_ptr() as i64);
		let vcode = builder.ins().iconst(ptr_type, code.as_ptr() as i64);
		let vprogbits = builder.ins().iconst(ptr_type, progbits.as_ptr() as i64);
		let vsidx = builder.declare_var(ptr_type);
		builder.def_var(vsidx, stack_idx_const);

		let mut valmap = FxHashMap::default();

		let clpush = |builder: &mut FunctionBuilder<'_>, value: Value| {
			let stidx = builder.use_var(vsidx);
			let new_st_idx = builder.ins().iadd_imm(stidx, CELL_SIZE);
			let slot_ptr = builder.ins().iadd(vstack, new_st_idx);
			builder.ins().store(aligned, value, slot_ptr, 0);
			builder.def_var(vsidx, new_st_idx);
		};

		let clpop = |builder: &mut FunctionBuilder<'_>, dep: isize| {
			let bb = builder.create_block();
			builder.append_block_param(bb, CELL_TYPE);

			let st_idx = builder.use_var(vsidx);
			if matches!(dep, 0) {
				let bbpop = builder.create_block();
				let icmp = builder.ins().icmp(IntCC::SignedLessThan, st_idx, null);
				builder.ins().brif(icmp, bb, &[zero.into()], bbpop, &[]);
				builder.switch_to_block(bbpop);
				let new_st_idx = builder.ins().iadd_imm(st_idx, -CELL_SIZE);
				let slot_ptr = builder.ins().iadd(vstack, new_st_idx);
				builder.def_var(vsidx, new_st_idx);
				let load_res = builder
					.ins()
					.load(CELL_TYPE, aligned, slot_ptr, CELL_SIZE as i32);
				builder.ins().jump(bb, &[load_res.into()]);
				builder.switch_to_block(bb);
				builder.block_params(bb)[0]
			} else {
				let new_st_idx = builder.ins().iadd_imm(st_idx, -CELL_SIZE);
				let slot_ptr = builder.ins().iadd(vstack, new_st_idx);
				builder.def_var(vsidx, new_st_idx);
				builder
					.ins()
					.load(CELL_TYPE, aligned, slot_ptr, CELL_SIZE as i32)
			}
		};

		let mut comp_stack = vec![0u32];
		let mut bbmap = FxHashMap::default();
		let mut jumpmap = Vec::<JumpEntry>::with_capacity(
			cfg.iter()
				.map(|op| match op.op {
					Op::Jz(..) => 1,
					Op::Jr(..) => 3,
					_ => 0,
				})
				.sum(),
		);

		let mut dep = *stack_idx;
		while let Some(n) = comp_stack.pop() {
			let op = &cfg[n as usize];

			macro_rules! push {
				($num:expr, $val:expr) => {
					if (op.depo & 1 << $num) != 0 {
						valmap.insert(n << 2 | $num, $val);
					} else {
						dep += 1;
						clpush(&mut builder, $val);
					}
				};
			}

			macro_rules! pop {
				($idx:expr) => {
					if let Some(&val) = op.depi.get($idx).and_then(|depi| valmap.get(depi)) {
						val
					} else {
						let val = clpop(&mut builder, dep);
						if dep > 0 {
							dep -= 1;
						}

						val
					}
				};
			}

			if op.block {
				if op.si.len() > 1 {
					dep = 0;
					if let Some(&bbref) = bbmap.get(&n) {
						if !block_filled {
							builder.ins().jump(bbref, &[]);
							block_filled = true;
						}

						continue;
					}
				}

				let new_bb = builder.create_block();
				if !block_filled {
					builder.ins().jump(new_bb, &[]);
				}

				block_filled = false;
				builder.switch_to_block(new_bb);
				bbmap.insert(n, new_bb);
			}

			debug_assert!(!block_filled);

			{
				let stidx = builder.use_var(vsidx);
				let stidx = builder.ins().sdiv_imm(stidx, CELL_SIZE);
				builder.ins().call(ps, &[vstack, stidx]);
			}

			match op.op {
				Op::Ld(val) => {
					let num = if matches!(val, 0) {
						zero
					} else {
						builder.ins().iconst(CELL_TYPE, val)
					};

					push!(0, num);
				}
				Op::Binary(bop) => {
					let b = pop!(0);
					let a = pop!(1);
					let num = match bop {
						BinaryOp::Add => builder.ins().iadd(a, b),
						BinaryOp::Sub => builder.ins().isub(a, b),
						BinaryOp::Mul => builder.ins().imul(a, b),
						BinaryOp::Div => {
							let div_bb = builder.create_block();
							let bb = builder.create_block();
							builder.append_block_param(bb, CELL_TYPE);
							builder.ins().brif(b, div_bb, &[], bb, &[zero.into()]);
							builder.switch_to_block(div_bb);
							let num = builder.ins().sdiv(a, b);
							builder.ins().jump(bb, &[num.into()]);
							builder.switch_to_block(bb);
							builder.block_params(bb)[0]
						}
						BinaryOp::Mod => {
							let div_bb = builder.create_block();
							let bb = builder.create_block();
							builder.append_block_param(bb, CELL_TYPE);
							builder.ins().brif(b, div_bb, &[], bb, &[zero.into()]);
							builder.switch_to_block(div_bb);
							let num = builder.ins().srem(a, b);
							builder.ins().jump(bb, &[num.into()]);
							builder.switch_to_block(bb);
							builder.block_params(bb)[0]
						}
						BinaryOp::Cmp => builder.ins().icmp(IntCC::SignedGreaterThan, a, b),
					};
					push!(0, num);
				}
				Op::Not => {
					let a = pop!(0);
					let eq = builder.ins().icmp_imm(IntCC::Equal, a, 0);
					push!(0, eq);
				}
				Op::Pop => {
					if op.depi.is_empty() {
						if matches!(dep, 0) {
							let bb = builder.create_block();
							builder.append_block_param(bb, ptr_type);
							let stidx = builder.use_var(vsidx);
							let new_st_idx = builder.ins().iadd_imm(stidx, -CELL_SIZE);
							let cmp = builder.ins().icmp(IntCC::SignedLessThan, stidx, null);
							builder
								.ins()
								.brif(cmp, bb, &[stidx.into()], bb, &[new_st_idx.into()]);
							builder.switch_to_block(bb);
							let new_st_idx = builder.block_params(bb)[0];
							builder.def_var(vsidx, new_st_idx);
						} else {
							dep -= 1;
							let stidx = builder.use_var(vsidx);
							let new_st_idx = builder.ins().iadd_imm(stidx, -CELL_SIZE);
							builder.def_var(vsidx, new_st_idx);
						}
					}
				}
				Op::Dup => {
					let a = pop!(0);
					push!(0, a);
					push!(1, a);
				}
				Op::Swp => {
					let b = pop!(0);
					let a = pop!(1);
					push!(0, b);
					push!(1, a);
				}
				Op::Rch => {
					let inst = builder.ins().call(get_char, &[]);
					let a = builder.inst_results(inst)[0];
					push!(0, a);
				}
				Op::Wch => {
					let a = pop!(0);
					builder.ins().call(put_char, &[a]);
				}
				Op::Rum => {
					let inst = builder.ins().call(get_num, &[]);
					let a = builder.inst_results(inst)[0];
					push!(0, a);
				}
				Op::Wum => {
					let a = pop!(0);
					builder.ins().call(put_num, &[a]);
				}
				Op::Rem(None) => {
					let b = pop!(0);
					let a = pop!(1);
					let idx_bb = builder.create_block();
					let bb = builder.create_block();
					builder.append_block_param(bb, CELL_TYPE);
					let a5 = builder.ins().ishl_imm(a, 5);
					let ab = builder.ins().bor(a5, b);
					let cmp =
						builder
							.ins()
							.icmp(IntCC::UnsignedLessThan, ab, two_five_six_zero_i32);
					builder.ins().brif(cmp, idx_bb, &[], bb, &[zero.into()]);
					builder.switch_to_block(idx_bb);
					let ab = if ptr_type.bits() > 32 {
						builder.ins().uextend(ptr_type, ab)
					} else {
						ab
					};

					let ab = builder.ins().imul_imm(ab, CELL_SIZE);
					let vcodeab = builder.ins().iadd(vcode, ab);
					let result = builder.ins().load(CELL_TYPE, aligned, vcodeab, 0);
					builder.ins().jump(bb, &[result.into()]);
					builder.switch_to_block(bb);
					let val = builder.block_params(bb)[0];
					push!(0, val);
				}
				Op::Rem(Some(off)) => {
					let result = builder
						.ins()
						.load(CELL_TYPE, aligned, vcode, i32::from(off) * 4);
					push!(0, result);
				}
				Op::Wem(xydir, None) => {
					let b = pop!(0);
					let a = pop!(1);
					let c = pop!(2);
					let bbwrite = builder.create_block();
					let bbexit = builder.create_block();
					let bb = builder.create_block();
					let a80 = builder.ins().icmp_imm(IntCC::UnsignedLessThan, a, 0);
					let b25 = builder.ins().icmp_imm(IntCC::UnsignedLessThan, b, 25);
					let ab8025 = builder.ins().band(a80, b25);
					builder.ins().brif(ab8025, bbwrite, &[], bb, &[]);
					builder.switch_to_block(bbwrite);
					let a5 = builder.ins().ishl_imm(a, 5);
					let ab = builder.ins().bor(a5, b);
					let ab = if ptr_type.bits() > 32 {
						builder.ins().uextend(ptr_type, ab)
					} else {
						ab
					};

					let ab4 = builder.ins().imul_imm(ab, CELL_SIZE);
					let vcodeab = builder.ins().iadd(vcode, ab4);
					builder.ins().store(aligned, c, vcodeab, 0);
					let ab3 = builder.ins().ushr_imm(ab, 3);
					let vprogbitsab3 = builder.ins().iadd(vprogbits, ab3);
					let progbitsread = builder.ins().load(types::I8, aligned, vprogbitsab3, 0);
					let ab7 = builder.ins().band_imm(ab, 7);
					let ab7 = builder.ins().ireduce(types::I8, ab7);
					let bit = builder.ins().ishl(one_i8, ab7);
					let bitcheck = builder.ins().band(progbitsread, bit);
					builder.ins().brif(bitcheck, bbexit, &[], bb, &[]);
					builder.switch_to_block(bbexit);
					let stidx = builder.use_var(vsidx);
					let stidx = builder.ins().sdiv_imm(stidx, CELL_SIZE);
					builder.ins().store(aligned, stidx, stack_idx_const, 0);
					let rstate = builder.ins().iconst(types::I32, i64::from(xydir));
					builder.ins().return_(&[rstate]);
					builder.switch_to_block(bb);
				}
				Op::Wem(xydir, Some(off)) => {
					let c = pop!(0);
					builder.ins().store(aligned, c, vcode, i32::from(off) * 4);
					if !matches!(progbits[off as usize >> 3] & 1 << (off & 7), 0) {
						let stidx = builder.use_var(vsidx);
						let stidx = builder.ins().sdiv_imm(stidx, CELL_SIZE);
						builder.ins().store(aligned, stidx, stack_idx_const, 0);
						let rstate = builder.ins().iconst(types::I32, i64::from(xydir));
						builder.ins().return_(&[rstate]);
						block_filled = true;
						continue;
					}
				}
				Op::Jr(ref rs) => {
					let [r0, r1, r2] = **rs;
					let inst = builder.ins().call(rand, &[]);
					let a = builder.inst_results(inst)[0];
					let cmp = builder.ins().icmp(IntCC::Equal, a, zero_i8);
					let bb = builder.create_block();
					let j = builder.ins().brif(cmp, Block::from_u32(0), &[], bb, &[]);
					jumpmap.push(JumpEntry::J1(j, r0));
					comp_stack.push(r0);
					builder.switch_to_block(bb);
					let cmp = builder.ins().icmp(IntCC::Equal, a, one_i8);
					let bb = builder.create_block();
					let j = builder.ins().brif(cmp, Block::from_u32(0), &[], bb, &[]);
					jumpmap.push(JumpEntry::J1(j, r1));
					comp_stack.push(r1);
					builder.switch_to_block(bb);
					let cmp = builder.ins().icmp(IntCC::Equal, a, two_i8);
					let j =
						builder
							.ins()
							.brif(cmp, Block::from_u32(1), &[], Block::from_u32(0), &[]);
					jumpmap.push(JumpEntry::J2(j, r2, op.n));
					comp_stack.push(r2);
					block_filled = true;
				}
				Op::Jz(rz) => {
					let a = pop!(0);
					let j = builder
						.ins()
						.brif(a, Block::from_u32(1), &[], Block::from_u32(0), &[]);
					jumpmap.push(JumpEntry::J2(j, op.n, rz));
					comp_stack.push(rz);
					block_filled = true;
				}
				Op::Ret => {
					builder.ins().return_(&[negative_one_i32]);
					block_filled = true;
					continue;
				}
				Op::Hcf => {
					let bb = builder.current_block().context("block not found")?;
					builder.ins().jump(bb, &[]);
					block_filled = true;
					continue;
				}
				_ => {}
			}
			comp_stack.push(op.n);
		}

		if !block_filled {
			builder.ins().return_(&[negative_one_i32]);
		}

		for jump in &jumpmap {
			match *jump {
				JumpEntry::J1(inst, loc) => builder.change_jump_destination(
					inst,
					Block::from_u32(0),
					*bbmap.get(&loc).context("block not found")?,
				),
				JumpEntry::J2(inst, loc_true, loc_false) => {
					builder.change_jump_destination(
						inst,
						Block::from_u32(1),
						*bbmap.get(&loc_true).context("block not found")?,
					);
					builder.change_jump_destination(
						inst,
						Block::from_u32(0),
						*bbmap.get(&loc_false).context("block not found")?,
					);
				}
			}
		}

		builder.seal_all_blocks();
	}

	fs::write(
		format!("../../out/unoptimized-{iteration}.clif"),
		ctx.func.to_string(),
	)?;

	ctx.optimize(&*isa, &mut ControlPlane::default())?;

	fs::write(
		format!("../../out/optimized-{iteration}.clif"),
		ctx.func.to_string(),
	)?;

	let id = module.declare_function("f", Linkage::Export, &ctx.func.signature)?;

	module.define_function(id, &mut ctx)?;
	module.clear_context(&mut ctx);
	module.finalize_definitions()?;

	let func = module.get_finalized_function(id);
	let result = unsafe { mem::transmute::<*const u8, fn() -> u32>(func)() };

	unsafe {
		module.free_memory();
	}

	Ok(result)
}

enum JumpEntry {
	J1(Inst, u32),
	J2(Inst, u32, u32),
}
