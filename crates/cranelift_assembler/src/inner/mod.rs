mod impls;

use std::{
	mem,
	ops::{Deref, DerefMut},
};

use cranelift_codegen::ir::{
	AbiParam, FuncRef, Function, InstBuilder as _, MemFlags, SourceLoc, StackSlot, StackSlotData,
	StackSlotKind, Type, types,
};
use cranelift_frontend::{FuncInstBuilder, FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::JITModule;
use cranelift_module::{Linkage, Module};
use frick_assembler::{AssemblyError, TAPE_SIZE};
use frick_ir::BrainIr;
use frick_utils::GetOrZero as _;

use super::CraneliftAssemblyError;

pub struct InnerAssembler<'a> {
	builder: FunctionBuilder<'a>,
	getchar: FuncRef,
	putchar: FuncRef,
	tape: StackSlot,
	ptr: StackSlot,
	ptr_type: Type,
}

impl<'a> InnerAssembler<'a> {
	pub fn new(
		func: &'a mut Function,
		fn_ctx: &'a mut FunctionBuilderContext,
		module: &mut JITModule,
		ptr_type: Type,
	) -> Result<Self, CraneliftAssemblyError> {
		let mut builder = FunctionBuilder::new(func, fn_ctx);

		let block = builder.create_block();

		builder.switch_to_block(block);

		let tape = {
			let data = StackSlotData::new(
				StackSlotKind::ExplicitSlot,
				(mem::size_of::<u8>() * TAPE_SIZE) as u32,
				0,
			);

			let slot = builder.create_sized_stack_slot(data);

			let buffer = builder.ins().stack_addr(ptr_type, slot, 0);

			builder.emit_small_memset(
				module.isa().frontend_config(),
				buffer,
				0,
				TAPE_SIZE as u64,
				0,
				MemFlags::new(),
			);

			slot
		};

		let ptr = {
			let data = StackSlotData::new(
				StackSlotKind::ExplicitSlot,
				mem::size_of::<usize>() as u32,
				0,
			);

			let slot = builder.create_sized_stack_slot(data);

			let zero = builder.ins().iconst(ptr_type, 0);

			builder.ins().stack_store(zero, slot, 0);

			slot
		};

		let putchar = {
			let mut putchar_sig = module.make_signature();
			putchar_sig.params.push(AbiParam::new(types::I32));
			putchar_sig.returns.push(AbiParam::new(types::I32));

			module.declare_function("rust_putchar", Linkage::Import, &putchar_sig)?
		};

		let getchar = {
			let mut getchar_sig = module.make_signature();
			getchar_sig.returns.push(AbiParam::new(types::I32));

			module.declare_function("rust_getchar", Linkage::Import, &getchar_sig)?
		};

		let putchar = module.declare_func_in_func(putchar, builder.func);
		let getchar = module.declare_func_in_func(getchar, builder.func);

		Ok(Self {
			builder,
			getchar,
			putchar,
			tape,
			ptr,
			ptr_type,
		})
	}

	pub fn assemble(
		mut self,
		ops: &[BrainIr],
	) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		self.ops(ops, 0)?;

		self.ins().return_(&[]);

		self.seal_all_blocks();

		self.builder.finalize();

		Ok(())
	}

	fn ops(
		&mut self,
		ops: &[BrainIr],
		mut op_count: u32,
	) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		for op in ops {
			self.set_srcloc(SourceLoc::new(op_count));

			match op {
				BrainIr::Boundary => continue,
				BrainIr::MovePointer(offset) => self.move_pointer(*offset),
				BrainIr::SetCell(value, offset) => {
					self.set_cell(*value, offset.get_or_zero());
				}
				BrainIr::ChangeCell(value, offset) => {
					self.change_cell(*value, offset.get_or_zero());
				}
				BrainIr::Output(options) => self.output(options)?,
				BrainIr::SetManyCells(options) => {
					self.set_many_cells(&options.values, options.start.get_or_zero());
				}
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
			}

			op_count += 1;
		}

		Ok(())
	}

	fn ins<'short>(&'short mut self) -> FuncInstBuilder<'short, 'a> {
		self.builder.ins()
	}
}

impl<'a> Deref for InnerAssembler<'a> {
	type Target = FunctionBuilder<'a>;

	fn deref(&self) -> &Self::Target {
		&self.builder
	}
}

impl DerefMut for InnerAssembler<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.builder
	}
}
