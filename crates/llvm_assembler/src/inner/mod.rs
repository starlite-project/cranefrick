mod impls;

use std::num::NonZero;

use frick_assembler::AssemblyError;
use frick_ir::BrainIr;
use inkwell::{
	AddressSpace,
	builder::Builder,
	context::Context,
	module::{Linkage, Module},
	values::{FunctionValue, IntValue, PointerValue},
};

use super::ContextExt;
use crate::LlvmAssemblyError;

pub struct InnerAssembler<'ctx> {
	pub context: &'ctx Context,
	pub module: Module<'ctx>,
	pub builder: Builder<'ctx>,
	pub functions: Functions<'ctx>,
	tape: PointerValue<'ctx>,
	ptr: PointerValue<'ctx>,
}

impl<'ctx> InnerAssembler<'ctx> {
	pub fn new(context: &'ctx Context) -> Self {
		let module = context.create_module("frick");
		let functions = Functions::new(context, &module);
		let builder = context.create_builder();

		let basic_block = context.append_basic_block(functions.main, "entry");
		builder.position_at_end(basic_block);

		let (tape, ptr) = {
			let ptr_type = context.default_ptr_type();
			let memory_size = context.i64_type().const_int(30_000, false);

			let tape = builder.build_alloca(ptr_type, "tape").unwrap();
			let ptr = builder.build_alloca(ptr_type, "ptr").unwrap();

			(tape, ptr)
		};

		Self {
			context,
			module,
			builder,
			functions,
			tape,
			ptr,
		}
	}

	pub fn assemble(
		mut self,
		ops: &[BrainIr],
	) -> Result<(Module<'ctx>, Functions<'ctx>), AssemblyError<LlvmAssemblyError>> {
		self.init_pointers()?;

		self.ops(ops)?;

		self.builder
			.build_return(None)
			.map_err(AssemblyError::backend)?;
		Ok(self.into_parts())
	}

	fn init_pointers(&self) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context.i8_type();
		let memory_size = self.context.i64_type().const_int(30_000, false);

		let data_ptr = self
			.builder
			.build_array_malloc(i8_type, memory_size, "alloc tape")?;

		self.builder.build_store(self.tape, data_ptr)?;
		self.builder.build_store(self.ptr, data_ptr)?;

		Ok(())
	}

	fn ops(&mut self, ops: &[BrainIr]) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		for op in ops {
			match op {
				BrainIr::SetCell(value, offset) => {
					self.set_cell(*value, offset.map_or(0, NonZero::get))?;
				}
				BrainIr::ChangeCell(value, offset) => {
					self.change_cell(*value, offset.map_or(0, NonZero::get))?;
				}
				BrainIr::SubCell(offset) => self.sub_cell(*offset)?,
				BrainIr::MovePointer(offset) => self.move_pointer(*offset)?,
				BrainIr::OutputCurrentCell => self.output_current_cell()?,
				BrainIr::OutputChar(c) => self.output_char(*c)?,
				BrainIr::OutputChars(c) => self.output_chars(c)?,
				BrainIr::InputIntoCell => self.input_into_cell()?,
				BrainIr::DynamicLoop(ops) => self.dynamic_loop(ops)?,
				BrainIr::IfNz(ops) => self.if_nz(ops)?,
				BrainIr::FindZero(offset) => self.find_zero(*offset)?,
				BrainIr::MoveValue(factor, offset) => self.move_value(*factor, *offset)?,
				BrainIr::TakeValue(factor, offset) => self.take_value(*factor, *offset)?,
				BrainIr::FetchValue(factor, offset) => self.fetch_value(*factor, *offset)?,
				BrainIr::ReplaceValue(factor, offset) => self.replace_value(*factor, *offset)?,
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
			}
		}

		Ok(())
	}

	fn into_parts(self) -> (Module<'ctx>, Functions<'ctx>) {
		(self.module, self.functions)
	}
}

#[derive(Clone, Copy)]
pub struct Functions<'ctx> {
	pub getchar: FunctionValue<'ctx>,
	pub putchar: FunctionValue<'ctx>,
	pub main: FunctionValue<'ctx>,
}

impl<'ctx> Functions<'ctx> {
	fn new(context: &'ctx Context, module: &Module<'ctx>) -> Self {
		let i8_type = context.i8_type();
		let ptr_type = context.default_ptr_type();
		let void_type = context.void_type();

		let getchar_ty = void_type.fn_type(&[ptr_type.into()], false);
		let getchar = module.add_function("getchar", getchar_ty, Some(Linkage::External));

		let putchar_ty = void_type.fn_type(&[i8_type.into()], false);
		let putchar = module.add_function("putchar", putchar_ty, Some(Linkage::External));

		let main_ty = void_type.fn_type(&[], false);
		let main = module.add_function("main", main_ty, Some(Linkage::External));

		Self {
			getchar,
			putchar,
			main,
		}
	}
}
