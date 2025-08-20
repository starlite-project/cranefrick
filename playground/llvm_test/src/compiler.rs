use std::collections::VecDeque;

use color_eyre::Result;
use inkwell::{
	AddressSpace, IntPredicate, OptimizationLevel,
	basic_block::BasicBlock,
	builder::{Builder, BuilderError},
	context::Context,
	module::{Linkage, Module},
	targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine},
	types::PointerType,
	values::{FunctionValue, PointerValue},
};
pub struct Compiler<'ctx> {
	pub context: &'ctx Context,
	pub module: Module<'ctx>,
	pub builder: Builder<'ctx>,
}

impl<'ctx> Compiler<'ctx> {
	pub fn init_targets() {
		Target::initialize_all(&InitializationConfig::default());
	}

	pub fn compile(&self, program: &str) -> BuilderResult {
		let mut while_blocks = VecDeque::new();
		let functions = self.init_functions();
		let (data, ptr) = self.build_main(functions)?;
		self.init_pointers(functions, data, ptr)?;

		let iter = program.chars();

		for c in iter {
			match c {
				'>' => {
					self.build_add_ptr(false, ptr)?;
				}
				'<' => {
					self.build_add_ptr(true, ptr)?;
				}
				'+' => self.build_add(true, ptr)?,
				'-' => self.build_add(false, ptr)?,
				',' => self.build_getchar(functions, ptr)?,
				'.' => self.build_putchar(functions, ptr)?,
				'[' => self.build_while_start(functions, ptr, &mut while_blocks)?,
				']' => self.build_while_end(&mut while_blocks)?,
				_ => {}
			}
		}

		self.return_zero()?;

		Ok(())
	}

	fn init_functions(&self) -> Functions<'ctx> {
		let i32_type = self.context.i32_type();
		let i64_type = self.context.i64_type();
		let ptr_type = self.context.ptr_type(AddressSpace::default());

		let calloc_fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
		let calloc = self
			.module
			.add_function("calloc", calloc_fn_type, Some(Linkage::External));

		let getchar_fn_type = i32_type.fn_type(&[], false);
		let getchar = self
			.module
			.add_function("getchar", getchar_fn_type, Some(Linkage::External));

		let putchar_fn_type = i32_type.fn_type(&[i32_type.into()], false);
		let putchar = self
			.module
			.add_function("putchar", putchar_fn_type, Some(Linkage::External));

		let main_fn_type = i32_type.fn_type(&[], false);
		let main = self
			.module
			.add_function("main", main_fn_type, Some(Linkage::External));

		Functions {
			calloc,
			getchar,
			putchar,
			main,
		}
	}

	fn build_main(
		&self,
		functions: Functions<'ctx>,
	) -> BuilderResult<(PointerValue<'ctx>, PointerValue<'ctx>)> {
		let basic_block = self.context.append_basic_block(functions.main, "entry");
		self.builder.position_at_end(basic_block);

		let ptr_type = self.ptr_type();

		let data = self.builder.build_alloca(ptr_type, "data")?;
		let ptr = self.builder.build_alloca(ptr_type, "data")?;

		Ok((data, ptr))
	}

	fn init_pointers(
		&self,
		functions: Functions<'ctx>,
		data: PointerValue<'ctx>,
		ptr: PointerValue<'ctx>,
	) -> BuilderResult {
		let i64_type = self.context.i64_type();
		let i64_memory_size = i64_type.const_int(30_000, false);
		let i64_element_size = i64_type.const_int(1, false);

		let data_ptr = self.builder.build_call(
			functions.calloc,
			&[i64_memory_size.into(), i64_element_size.into()],
			"calloc_call",
		)?;

		let data_ptr_result: Result<_, _> = data_ptr.try_as_basic_value().flip().into();

		let data_ptr_basic_val = data_ptr_result.unwrap();

		self.builder.build_store(data, data_ptr_basic_val)?;
		self.builder.build_store(ptr, data_ptr_basic_val)?;

		Ok(())
	}

	fn build_add_ptr(&self, left: bool, ptr: PointerValue<'ctx>) -> BuilderResult {
		let ptr_type = self.ptr_type();
		let i32_type = self.context.i32_type();
		let i32_amount = i32_type.const_int(if left { -1i32 } else { 1i32 } as u64, false);
		let ptr_load = self
			.builder
			.build_load(ptr_type, ptr, "load ptr")?
			.into_pointer_value();

		let result = unsafe {
			self.builder
				.build_in_bounds_gep(ptr_type, ptr_load, &[i32_amount], "add to pointer")
		}?;

		self.builder.build_store(ptr, result)?;

		Ok(())
	}

	fn build_add(&self, positive: bool, ptr: PointerValue<'ctx>) -> BuilderResult {
		let ptr_type = self.ptr_type();
		let i8_type = self.context.i8_type();
		let i8_amount = i8_type.const_int(if positive { 1i32 } else { -1i32 } as u64, false);
		let ptr_load = self
			.builder
			.build_load(ptr_type, ptr, "load ptr")?
			.into_pointer_value();
		let ptr_value = self
			.builder
			.build_load(i8_type, ptr_load, "load ptr value")?;

		let result =
			self.builder
				.build_int_add(ptr_value.into_int_value(), i8_amount, "add to data ptr")?;
		self.builder.build_store(ptr_load, result)?;

		Ok(())
	}

	fn build_getchar(&self, functions: Functions<'ctx>, ptr: PointerValue<'ctx>) -> BuilderResult {
		let getchar_call = self
			.builder
			.build_call(functions.getchar, &[], "getchar call")?;
		let getchar_result: Result<_, _> = getchar_call.try_as_basic_value().flip().into();
		let getchar_basicvalue = getchar_result.unwrap().into_int_value();
		let i8_type = self.context.i8_type();
		let truncated = self.builder.build_int_truncate(
			getchar_basicvalue,
			i8_type,
			"getchar truncate result",
		)?;

		self.builder.build_store(ptr, truncated)?;

		Ok(())
	}

	fn build_putchar(&self, functions: Functions<'ctx>, ptr: PointerValue<'ctx>) -> BuilderResult {
		let i8_type = self.context.i8_type();
		let ptr_type = self.ptr_type();
		let char_to_put = self.builder.build_load(
			i8_type,
			self.builder
				.build_load(ptr_type, ptr, "load ptr value")?
				.into_pointer_value(),
			"load ptr ptr value",
		)?;

		let s_ext = self.builder.build_int_s_extend(
			char_to_put.into_int_value(),
			self.context.i32_type(),
			"putchar sign extend",
		)?;

		self.builder
			.build_call(functions.putchar, &[s_ext.into()], "putchar call")?;

		Ok(())
	}

	fn build_while_start(
		&self,
		functions: Functions<'ctx>,
		ptr: PointerValue<'ctx>,
		while_blocks: &mut VecDeque<WhileBlock<'ctx>>,
	) -> BuilderResult {
		let num_while_blocks = while_blocks.len() + 1;
		let while_block = WhileBlock {
			start: self.context.append_basic_block(
				functions.main,
				format!("while_start {num_while_blocks}").as_str(),
			),
			body: self.context.append_basic_block(
				functions.main,
				format!("while_body {num_while_blocks}").as_str(),
			),
			end: self.context.append_basic_block(
				functions.main,
				format!("while_end {num_while_blocks}").as_str(),
			),
		};

		while_blocks.push_front(while_block);

		self.builder.build_unconditional_branch(while_block.start)?;

		self.builder.position_at_end(while_block.start);

		let i8_type = self.context.i8_type();
		let i8_zero = i8_type.const_int(0, false);
		let ptr_type = self.ptr_type();
		let ptr_load = self
			.builder
			.build_load(ptr_type, ptr, "load ptr")?
			.into_pointer_value();
		let ptr_value = self
			.builder
			.build_load(i8_type, ptr_load, "load ptr value")?
			.into_int_value();

		let cmp = self.builder.build_int_compare(
			IntPredicate::NE,
			ptr_value,
			i8_zero,
			"compare value at pointer to zero",
		)?;

		self.builder
			.build_conditional_branch(cmp, while_block.body, while_block.end)?;

		self.builder.position_at_end(while_block.body);

		Ok(())
	}

	fn build_while_end(&self, while_blocks: &mut VecDeque<WhileBlock<'ctx>>) -> BuilderResult {
		if let Some(while_block) = while_blocks.pop_front() {
			self.builder.build_unconditional_branch(while_block.start)?;
			self.builder.position_at_end(while_block.end);
			Ok(())
		} else {
			panic!("unmatched brackets");
		}
	}

	fn build_free(&self, data: PointerValue<'ctx>) -> BuilderResult {
		let ptr_type = self.ptr_type();

		self.builder.build_free(
			self.builder
				.build_load(ptr_type, data, "load")?
				.into_pointer_value(),
		)?;
		Ok(())
	}

	fn return_zero(&self) -> BuilderResult {
		let i32_type = self.context.i32_type();
		let i32_zero = i32_type.const_zero();
		self.builder.build_return(Some(&i32_zero))?;

		Ok(())
	}

	fn ptr_type(&self) -> PointerType<'ctx> {
		self.context.ptr_type(AddressSpace::default())
	}
}

#[derive(Clone, Copy)]
struct Functions<'ctx> {
	calloc: FunctionValue<'ctx>,
	getchar: FunctionValue<'ctx>,
	putchar: FunctionValue<'ctx>,
	main: FunctionValue<'ctx>,
}

#[derive(Clone, Copy)]
struct WhileBlock<'ctx> {
	start: BasicBlock<'ctx>,
	body: BasicBlock<'ctx>,
	end: BasicBlock<'ctx>,
}

type BuilderResult<T = ()> = Result<T, BuilderError>;
