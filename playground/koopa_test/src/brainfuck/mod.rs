#![expect(clippy::mut_mut)]

use anyhow::{Context as _, Result};
use koopa::ir::{
	BasicBlock, BinaryOp, Function, FunctionData, Program, Type, Value,
	builder::{BasicBlockBuilder as _, GlobalInstBuilder, LocalInstBuilder as _, ValueBuilder},
};

struct Environment<'a> {
	ptr: Value,
	putchar: Function,
	getchar: Function,
	main: &'a mut FunctionData,
}

pub fn build_program(src: &str) -> Result<Program> {
	let mut program = Program::new();
	let zero = program
		.new_value()
		.zero_init(Type::get_array(Type::get_i32(), 30_000));

	let ptr = program.new_value().global_alloc(zero);
	program.set_value_name(ptr, Some("@tape".to_owned()));

	let putchar = FunctionData::new_decl(
		"@putchar".to_owned(),
		vec![Type::get_i32()],
		Type::get_i32(),
	);
	let putchar = program.new_func(putchar);
	let getchar = FunctionData::new_decl("@getchar".to_owned(), Vec::new(), Type::get_i32());
	let getchar = program.new_func(getchar);

	let main = FunctionData::new("@main".to_owned(), Vec::new(), Type::get_i32());
	let main = program.new_func(main);

	generate_main(
		src,
		Environment {
			ptr,
			putchar,
			getchar,
			main: program.func_mut(main),
		},
	)?;

	Ok(program)
}

macro_rules! new_bb {
	($func:expr) => {
		$func.dfg_mut().new_bb()
	};
}

macro_rules! new_value {
	($func:expr) => {
		$func.dfg_mut().new_value()
	};
}

macro_rules! add_bb {
	($func:expr, $bb:expr) => {
		$func
			.layout_mut()
			.bbs_mut()
			.push_key_back($bb)
			.ok()
			.context("failed to push bb")
	};
}

macro_rules! add_inst {
	($func:expr, $bb:expr, $inst:expr) => {
		$func
			.layout_mut()
			.bb_mut($bb)
			.insts_mut()
			.push_key_back($inst)
			.ok()
			.context("failed to push inst")
	};
}

fn generate_main(src: &str, mut env: Environment<'_>) -> Result<()> {
	let main = &mut env.main;
	let entry = new_bb!(main).basic_block(Some("%entry".to_owned()));
	add_bb!(main, entry)?;

	let ptr = new_value!(main).alloc(Type::get_pointer(Type::get_i32()));
	main.dfg_mut().set_value_name(ptr, Some("%ptr".to_owned()));
	add_inst!(main, entry, ptr)?;

	let zero = new_value!(main).integer(0);
	let data_ptr = new_value!(main).get_elem_ptr(env.ptr, zero);
	add_inst!(main, entry, data_ptr)?;
	let store = new_value!(main).store(data_ptr, ptr);
	add_inst!(main, entry, store)?;
	env.ptr = ptr;

	let bb = generate_bbs(src, &mut env, entry)?;

	let main = &mut env.main;
	let end = new_bb!(main).basic_block(Some("%end".into()));
	add_bb!(main, end)?;
	let jump = new_value!(main).jump(end);
	add_inst!(main, bb, jump)?;
	let ret = new_value!(main).ret(Some(zero));
	add_inst!(main, end, ret)?;

	Ok(())
}

fn generate_bbs(src: &str, env: &mut Environment<'_>, entry: BasicBlock) -> Result<BasicBlock> {
	let mut bb = new_bb!(env.main).basic_block(None);
	add_bb!(env.main, bb)?;
	let jump = new_value!(env.main).jump(bb);
	add_inst!(env.main, entry, jump)?;
	let mut loop_info = Vec::new();
	for result in src.bytes() {
		bb = match result {
			b'>' => generate_ptr_op(env, bb, 1)?,
			b'<' => generate_ptr_op(env, bb, -1)?,
			b'+' => generate_data_op(env, bb, 1)?,
			b'-' => generate_data_op(env, bb, -1)?,
			b'[' => generate_start(env, bb, &mut loop_info)?,
			b']' => generate_end(env, bb, &mut loop_info)?,
			b'.' => generate_put(env, bb)?,
			b',' => generate_get(env, bb)?,
			_ => continue,
		}
	}

	Ok(bb)
}

fn generate_ptr_op(env: &mut Environment<'_>, bb: BasicBlock, i: i32) -> Result<BasicBlock> {
	let main = &mut env.main;
	let load = new_value!(main).load(env.ptr);
	add_inst!(main, bb, load)?;
	let index = new_value!(main).integer(i);
	let gp = new_value!(main).get_ptr(load, index);
	add_inst!(main, bb, gp)?;
	let store = new_value!(main).store(gp, env.ptr);
	add_inst!(main, bb, store)?;

	Ok(bb)
}

fn generate_data_op(env: &mut Environment<'_>, bb: BasicBlock, i: i32) -> Result<BasicBlock> {
	let main = &mut env.main;
	let load = new_value!(main).load(env.ptr);
	add_inst!(main, bb, load)?;
	let data = new_value!(main).load(load);
	add_inst!(main, bb, data)?;
	let rhs = new_value!(main).integer(i);
	let add = new_value!(main).binary(BinaryOp::Add, data, rhs);
	add_inst!(main, bb, add)?;
	let store = new_value!(main).store(add, load);
	add_inst!(main, bb, store)?;

	Ok(bb)
}

fn generate_start(
	env: &mut Environment<'_>,
	bb: BasicBlock,
	loop_info: &mut Vec<(BasicBlock, BasicBlock)>,
) -> Result<BasicBlock> {
	let main = &mut env.main;

	let cond_bb = new_bb!(main).basic_block(Some("%while_cond".into()));
	add_bb!(main, cond_bb)?;
	let jump = new_value!(main).jump(cond_bb);
	add_inst!(main, bb, jump)?;

	let load = new_value!(main).load(env.ptr);
	add_inst!(main, cond_bb, load)?;
	let data = new_value!(main).load(load);
	add_inst!(main, cond_bb, data)?;
	let zero = new_value!(main).integer(0);
	let cmp = new_value!(main).binary(BinaryOp::NotEq, data, zero);
	add_inst!(main, cond_bb, cmp)?;

	let body_bb = new_bb!(main).basic_block(Some("%while_body".into()));
	let end_bb = new_bb!(main).basic_block(Some("%while_end".into()));
	let br = new_value!(main).branch(cmp, body_bb, end_bb);
	add_inst!(main, cond_bb, br)?;
	add_bb!(main, body_bb)?;

	loop_info.push((cond_bb, end_bb));
	Ok(body_bb)
}

fn generate_end(
	env: &mut Environment<'_>,
	bb: BasicBlock,
	loop_info: &mut Vec<(BasicBlock, BasicBlock)>,
) -> Result<BasicBlock> {
	let (cond_bb, end_bb) = loop_info.pop().context("mismatch brackets")?;
	let jump = new_value!(env.main).jump(cond_bb);
	add_inst!(env.main, bb, jump)?;
	add_bb!(env.main, end_bb)?;
	Ok(end_bb)
}

fn generate_put(env: &mut Environment<'_>, bb: BasicBlock) -> Result<BasicBlock> {
	let main = &mut env.main;
	let load = new_value!(main).load(env.ptr);
	add_inst!(main, bb, load)?;
	let data = new_value!(main).load(load);
	add_inst!(main, bb, data)?;
	let call = new_value!(main).call(env.putchar, vec![data]);
	add_inst!(main, bb, call)?;

	Ok(bb)
}

fn generate_get(env: &mut Environment<'_>, bb: BasicBlock) -> Result<BasicBlock> {
	let main = &mut env.main;
	let call = new_value!(main).call(env.getchar, Vec::new());
	add_inst!(main, bb, call)?;
	let load = new_value!(main).load(env.ptr);
	add_inst!(main, bb, load)?;
	let store = new_value!(main).store(call, load);
	add_inst!(main, bb, store)?;

	Ok(bb)
}
