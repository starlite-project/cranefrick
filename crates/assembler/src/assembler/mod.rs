mod impls;

use std::{
	num::NonZero,
	ops::{Deref, DerefMut},
};

use cranefrick_mlir::{BrainMlir, Compiler};
use cranelift_codegen::ir::{
	AbiParam, Fact, FuncRef, Function, InstBuilder as _, Type, Value, types,
};
use cranelift_frontend::{FuncInstBuilder, FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::JITModule;
use cranelift_module::{Linkage, Module as _};

use crate::AssemblyError;

pub struct Assembler<'a> {
	ptr_type: Type,
	builder: FunctionBuilder<'a>,
	read: FuncRef,
	write: FuncRef,
	memory_address: Value,
	last_value: Option<(i32, Value)>,
}

impl<'a> Assembler<'a> {
	pub fn new(
		func: &'a mut Function,
		fn_ctx: &'a mut FunctionBuilderContext,
		module: &mut JITModule,
		ptr_type: Type,
	) -> Result<Self, AssemblyError> {
		let mut builder = FunctionBuilder::new(func, fn_ctx);

		let block = builder.create_block();

		builder.switch_to_block(block);
		builder.append_block_params_for_function_params(block);

		let memory_address = builder.block_params(block)[0];

		{
			let memory_type = builder
				.func
				.stencil
				.create_memory_type(cranelift_codegen::ir::MemoryTypeData::Memory { size: 30_000 });

			let fact = Fact::Mem {
				ty: memory_type,
				min_offset: 0,
				max_offset: 30_000,
				nullable: false,
			};

			builder.func.dfg.facts[memory_address] = Some(fact);
		}

		let write = {
			let mut write_sig = module.make_signature();
			write_sig.params.push(AbiParam::new(types::I8));
			module.declare_function("write", Linkage::Import, &write_sig)?
		};

		let read = {
			let mut read_sig = module.make_signature();
			read_sig.params.push(AbiParam::new(ptr_type));
			module.declare_function("read", Linkage::Import, &read_sig)?
		};

		let write = module.declare_func_in_func(write, builder.func);
		let read = module.declare_func_in_func(read, builder.func);

		Ok(Self {
			ptr_type,
			builder,
			read,
			write,
			memory_address,
			last_value: None,
		})
	}

	#[tracing::instrument("creating cranelift ir", skip_all)]
	pub fn assemble(mut self, compiler: Compiler) -> Result<(), AssemblyError> {
		self.ops(&compiler)?;

		self.ins().return_(&[]);

		self.seal_all_blocks();

		self.builder.finalize();

		Ok(())
	}

	fn ops(&mut self, ops: &[BrainMlir]) -> Result<(), AssemblyError> {
		for op in ops {
			match op {
				BrainMlir::ChangeCell(i, offset) => {
					self.change_cell(*i, offset.map_or(0, NonZero::get));
				}
				BrainMlir::MovePointer(offset) => self.move_pointer(*offset),
				BrainMlir::DynamicLoop(ops) => self.dynamic_loop(ops)?,
				BrainMlir::OutputCurrentCell => self.output_current_cell(),
				BrainMlir::OutputChar(c) => self.output_char(*c),
				BrainMlir::InputIntoCell => self.input_into_cell(),
				BrainMlir::SetCell(value, offset) => {
					self.set_cell(*value, offset.map_or(0, NonZero::get));
				}
				BrainMlir::MoveValue(factor, offset) => self.move_value(*factor, *offset),
				BrainMlir::TakeValue(factor, offset) => self.take_value(*factor, *offset),
				BrainMlir::FetchValue(factor, offset) => self.fetch_value(*factor, *offset),
				BrainMlir::IfNz(ops) => self.if_nz(ops)?,
				BrainMlir::FindZero(offset) => self.find_zero(*offset),
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
			}
		}

		Ok(())
	}

	#[inline]
	fn ins<'short>(&'short mut self) -> FuncInstBuilder<'short, 'a> {
		self.builder.ins()
	}

	fn const_u8(&mut self, value: u8) -> Value {
		self.ins().iconst(types::I8, i64::from(value))
	}
}

impl<'a> Deref for Assembler<'a> {
	type Target = FunctionBuilder<'a>;

	fn deref(&self) -> &Self::Target {
		&self.builder
	}
}

impl DerefMut for Assembler<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.builder
	}
}
