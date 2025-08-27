mod impls;
mod srcloc;

use std::{
	collections::HashMap,
	num::NonZero,
	ops::{Deref, DerefMut},
};

use cranelift_codegen::ir::{
	AbiParam, Fact, FuncRef, Function, InstBuilder as _, MemoryTypeData, SourceLoc, StackSlotData,
	StackSlotKind, Type, Value, types,
};
use cranelift_frontend::{FuncInstBuilder, FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::JITModule;
use cranelift_module::{Linkage, Module};
use frick_assembler::AssemblyError;
use frick_ir::BrainIr;

use self::srcloc::SrcLoc;
use super::CraneliftAssemblyError;

pub struct InnerAssembler<'a> {
	builder: FunctionBuilder<'a>,
	read: FuncRef,
	write: FuncRef,
	current_srcloc: SrcLoc,
	ptr_variable: Variable,
	loads: HashMap<i32, Value>,
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

		let tape_stack_slot = builder.create_sized_stack_slot(StackSlotData::new(
			StackSlotKind::ExplicitSlot,
			30_000,
			0,
		));

		let ptr_variable = {
			let ptr_value = builder.declare_var(ptr_type);
			let stack_slot = builder.ins().stack_addr(ptr_type, tape_stack_slot, 0);

			let memory_type = builder
				.func
				.create_memory_type(MemoryTypeData::Memory { size: 30_000 });

			let fact = Fact::Mem {
				ty: memory_type,
				min_offset: 0,
				max_offset: 30_000,
				nullable: false,
			};

			builder.func.dfg.facts[stack_slot] = Some(fact);

			builder.def_var(ptr_value, stack_slot);

			ptr_value
		};

		let write = {
			let mut write_sig = module.make_signature();
			write_sig.params.push(AbiParam::new(types::I32));

			module.declare_function("write", Linkage::Import, &write_sig)
		}?;

		let read = {
			let mut read_sig = module.make_signature();
			read_sig.params.push(AbiParam::new(ptr_type));

			module.declare_function("read", Linkage::Import, &read_sig)
		}?;

		let write = module.declare_func_in_func(write, builder.func);
		let read = module.declare_func_in_func(read, builder.func);

		Ok(Self {
			current_srcloc: SrcLoc::empty(),
			builder,
			read,
			write,
			ptr_variable,
			loads: HashMap::new(),
		})
	}

	pub fn assemble(
		mut self,
		ops: &[BrainIr],
	) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		self.ops(ops)?;

		self.ins().return_(&[]);

		self.seal_all_blocks();

		self.builder.finalize();

		Ok(())
	}

	fn ops(&mut self, ops: &[BrainIr]) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		for op in ops {
			match op {
				BrainIr::MovePointer(offset) => self.move_pointer(*offset),
				BrainIr::SetCell(value, offset) => {
					self.set_cell(*value, offset.map_or(0, NonZero::get));
				}
				BrainIr::ChangeCell(value, offset) => {
					self.change_cell(*value, offset.map_or(0, NonZero::get));
				}
				BrainIr::SubCell(offset) => self.sub_cell(*offset),
				BrainIr::OutputCurrentCell => self.output_current_cell(),
				BrainIr::OutputChar(c) => self.output_char(*c),
				BrainIr::OutputChars(c) => self.output_chars(c),
				BrainIr::InputIntoCell => self.input_into_cell(),
				BrainIr::DynamicLoop(ops) => self.dynamic_loop(ops)?,
				BrainIr::IfNotZero(ops) => self.if_not_zero(ops)?,
				BrainIr::FindZero(offset) => self.find_zero(*offset),
				BrainIr::MoveValueTo(factor, offset) => self.move_value_to(*factor, *offset),
				BrainIr::TakeValueTo(factor, offset) => self.take_value_to(*factor, *offset),
				BrainIr::FetchValueFrom(factor, offset) => self.fetch_value_from(*factor, *offset),
				BrainIr::ReplaceValueFrom(factor, offset) => {
					self.replace_value_from(*factor, *offset);
				}
				BrainIr::ScaleValue(factor) => self.scale_value(*factor),
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
			}
		}

		Ok(())
	}

	fn ins<'short>(&'short mut self) -> FuncInstBuilder<'short, 'a> {
		self.builder.ins()
	}

	fn ptr_value(&mut self) -> Value {
		let variable = self.ptr_variable;

		self.use_var(variable)
	}

	fn const_u8(&mut self, value: u8) -> Value {
		self.ins().iconst(types::I8, i64::from(value))
	}

	fn add_srcflag(&mut self, flag: SrcLoc) {
		self.current_srcloc |= flag;

		let srcloc = self.current_srcloc.bits();

		self.set_srcloc(SourceLoc::new(srcloc));
	}

	fn remove_srcflag(&mut self, flag: SrcLoc) {
		self.current_srcloc &= !flag;

		let srcloc = self.current_srcloc.bits();

		self.set_srcloc(SourceLoc::new(srcloc));
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
