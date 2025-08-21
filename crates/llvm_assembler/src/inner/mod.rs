mod impls;

use std::num::NonZero;

use frick_assembler::AssemblyError;
use frick_ir::BrainIr;
use inkwell::{
	builder::Builder,
	context::Context,
	module::{Linkage, Module},
	values::{FunctionValue, PointerValue},
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
			// let ptr_type = context.default_ptr_type();
			// let memory_size = context.i64_type().const_int(30_000, false);

			// let tape = builder.build_alloca(ptr_type, "tape").unwrap();
			// // let tape = builder.build_array_alloca(context.i8_type(), memory_size, "tape").unwrap();

			// let ptr = builder.build_alloca(ptr_type, "ptr").unwrap();

			// (tape, ptr)

			let ptr_type =context.default_ptr_type();
			let i8_type = context.i8_type();
			let i8_array_type = i8_type.array_type(30_000);

			let tape_global_value = module.add_global(i8_array_type, None, "tape");

			let zero_array = i8_array_type.const_zero();

			tape_global_value.set_initializer(&zero_array);

			let tape = tape_global_value.as_pointer_value();

			(tape, tape)
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
		self,
		ops: &[BrainIr],
	) -> Result<(Module<'ctx>, Functions<'ctx>), AssemblyError<LlvmAssemblyError>> {
		self.init_pointers()?;

		self.ops(ops)?;

		// self.builder
		// 	.build_free(
		// 		self.builder
		// 			.build_load(self.context.default_ptr_type(), self.tape, "load")
		// 			.map_err(AssemblyError::backend)?
		// 			.into_pointer_value(),
		// 	)
		// 	.map_err(AssemblyError::backend)?;

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
			.build_malloc(i8_type.array_type(30_000), "alloc tape")?;
			// .build_array_malloc(i8_type, memory_size, "alloc tape")?;

		self.builder.build_store(self.tape, data_ptr)?;
		self.builder.build_store(self.ptr, data_ptr)?;

		Ok(())
	}

	fn ops(&self, ops: &[BrainIr]) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		for op in ops {
			match op {
				BrainIr::MovePointer(offset) => self.move_pointer(*offset)?,
				BrainIr::SetCell(value, offset) => {
					self.set_cell(*value, offset.map_or(0, NonZero::get))?;
				}
				BrainIr::ChangeCell(value, offset) => {
					self.change_cell(*value, offset.map_or(0, NonZero::get))?;
				}
				BrainIr::OutputCurrentCell => self.output_current_cell()?,
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
				// _ => {}
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
