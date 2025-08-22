mod impls;

use std::num::NonZero;

use frick_assembler::AssemblyError;
use frick_ir::BrainIr;
use inkwell::{
	attributes::{Attribute, AttributeLoc},
	builder::Builder,
	context::Context,
	module::{Linkage, Module},
	types::IntType,
	values::{FunctionValue, PointerValue},
};

use super::{ContextExt, LlvmAssemblyError};

pub struct InnerAssembler<'ctx> {
	pub context: &'ctx Context,
	pub module: Module<'ctx>,
	pub builder: Builder<'ctx>,
	pub functions: Functions<'ctx>,
	tape: PointerValue<'ctx>,
	ptr: PointerValue<'ctx>,
	ptr_type: IntType<'ctx>,
}

impl<'ctx> InnerAssembler<'ctx> {
	pub fn new(context: &'ctx Context) -> Result<Self, LlvmAssemblyError> {
		let module = context.create_module("frick");
		let functions = Functions::new(context, &module);
		let builder = context.create_builder();

		let basic_block = context.append_basic_block(functions.main, "entry");
		builder.position_at_end(basic_block);

		let i32_type = context.i32_type();
		let tape = {
			let i8_type = context.i8_type();
			let i8_array_type = i8_type.array_type(30_000);

			let tape_alloca = builder.build_alloca(i8_array_type, "tape")?;

			builder.build_store(tape_alloca, i8_type.array_type(30_000).const_zero())?;

			tape_alloca
		};

		let ptr = {
			let ptr_alloca = builder.build_alloca(i32_type, "ptr")?;

			builder.build_store(ptr_alloca, i32_type.const_zero())?;

			ptr_alloca
		};

		Ok(Self {
			context,
			module,
			builder,
			functions,
			tape,
			ptr,
			ptr_type: i32_type,
		})
	}

	pub fn assemble(
		self,
		ops: &[BrainIr],
	) -> Result<(Module<'ctx>, Functions<'ctx>), AssemblyError<LlvmAssemblyError>> {
		self.ops(ops)?;

		self.builder
			.build_return(None)
			.map_err(AssemblyError::backend)?;
		Ok(self.into_parts())
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
				BrainIr::SubCell(offset) => self.sub_cell(*offset)?,
				BrainIr::OutputCurrentCell => self.output_current_cell()?,
				BrainIr::OutputChar(c) => self.output_char(*c)?,
				BrainIr::OutputChars(c) => self.output_chars(c)?,
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

		let nounwind_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nounwind"), 0);

		let noundef_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("noundef"), 1);
		let noalias_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("noalias"), 2);
		let nofree_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nofree"), 3);
		let nonnull_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nonnull"), 4);

		let getchar_ty = void_type.fn_type(&[ptr_type.into()], false);
		let getchar = module.add_function("getchar", getchar_ty, Some(Linkage::External));

		getchar.add_attribute(AttributeLoc::Function, nounwind_attr);
		getchar.add_attribute(AttributeLoc::Param(0), noundef_attr);
		getchar.add_attribute(AttributeLoc::Param(0), noalias_attr);
		getchar.add_attribute(AttributeLoc::Param(0), nofree_attr);
		getchar.add_attribute(AttributeLoc::Param(0), nonnull_attr);

		let putchar_ty = void_type.fn_type(&[i8_type.into()], false);
		let putchar = module.add_function("putchar", putchar_ty, Some(Linkage::External));

		putchar.add_attribute(AttributeLoc::Function, nounwind_attr);
		putchar.add_attribute(AttributeLoc::Param(0), noundef_attr);

		let main_ty = void_type.fn_type(&[], false);
		let main = module.add_function("main", main_ty, Some(Linkage::External));

		Self {
			getchar,
			putchar,
			main,
		}
	}
}
