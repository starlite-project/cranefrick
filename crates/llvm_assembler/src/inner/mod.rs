mod impls;

use frick_assembler::{AssemblyError, TAPE_SIZE};
use frick_ir::BrainIr;
use frick_utils::GetOrZero as _;
use inkwell::{
	attributes::{Attribute, AttributeLoc},
	builder::Builder,
	context::Context,
	intrinsics::Intrinsic,
	module::{Linkage, Module},
	types::IntType,
	values::{FunctionValue, PointerValue},
};

use super::{ContextExt, LlvmAssemblyError};

#[allow(dead_code)]
pub struct InnerAssembler<'ctx> {
	context: &'ctx Context,
	module: Module<'ctx>,
	builder: Builder<'ctx>,
	functions: Functions<'ctx>,
	tape: PointerValue<'ctx>,
	ptr: PointerValue<'ctx>,
	ptr_int_type: IntType<'ctx>,
}

impl<'ctx> InnerAssembler<'ctx> {
	pub fn new(context: &'ctx Context) -> Result<Self, LlvmAssemblyError> {
		let module = context.create_module("frick");
		let functions = Functions::new(context, &module)?;
		let builder = context.create_builder();

		let basic_block = context.append_basic_block(functions.main, "entry");
		builder.position_at_end(basic_block);

		let ptr_int_type = context.i64_type();
		let tape = {
			let i8_type = context.i8_type();
			let i8_array_type = i8_type.array_type(TAPE_SIZE as u32);
			let array_size = ptr_int_type.const_int(TAPE_SIZE as u64, false);

			let tape_alloca = builder.build_alloca(i8_array_type, "tape")?;

			builder.build_memset(tape_alloca, 1, i8_type.const_zero(), array_size)?;

			tape_alloca
		};

		let ptr = {
			let ptr_alloca = builder.build_alloca(ptr_int_type, "ptr")?;

			builder.build_store(ptr_alloca, ptr_int_type.const_zero())?;

			ptr_alloca
		};

		let zero = {
			let i64_type = context.i64_type();

			i64_type.const_zero()
		};

		builder.build_call(functions.lifetime_start, &[zero.into(), tape.into()], "")?;

		builder.build_call(functions.lifetime_start, &[zero.into(), ptr.into()], "")?;

		Ok(Self {
			context,
			module,
			builder,
			functions,
			tape,
			ptr,
			ptr_int_type,
		})
	}

	pub fn assemble(
		self,
		ops: &[BrainIr],
	) -> Result<(Module<'ctx>, Functions<'ctx>), AssemblyError<LlvmAssemblyError>> {
		self.ops(ops)?;

		let zero = {
			let i64_type = self.context.i64_type();

			i64_type.const_zero()
		};

		self.builder
			.build_call(
				self.functions.lifetime_end,
				&[zero.into(), self.tape.into()],
				"",
			)
			.map_err(AssemblyError::backend)?;
		self.builder
			.build_call(
				self.functions.lifetime_end,
				&[zero.into(), self.ptr.into()],
				"",
			)
			.map_err(AssemblyError::backend)?;

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
					self.set_cell(*value, offset.get_or_zero())?;
				}
				BrainIr::ChangeCell(value, offset) => {
					self.change_cell(*value, offset.get_or_zero())?;
				}
				BrainIr::SubCell(offset) => self.sub_cell(*offset)?,
				BrainIr::OutputCell {
					value_offset: value,
					offset,
				} => {
					self.output_current_cell(value.get_or_zero(), offset.get_or_zero())?;
				}
				BrainIr::OutputChar(c) => self.output_char(*c)?,
				BrainIr::OutputChars(c) => self.output_chars(c)?,
				BrainIr::InputIntoCell => self.input_into_cell()?,
				BrainIr::DynamicLoop(ops) => self.dynamic_loop(ops)?,
				BrainIr::IfNotZero(ops) => self.if_not_zero(ops)?,
				BrainIr::FindZero(offset) => self.find_zero(*offset)?,
				BrainIr::MoveValueTo(factor, offset) => self.move_value_to(*factor, *offset)?,
				BrainIr::TakeValueTo(factor, offset) => self.take_value_to(*factor, *offset)?,
				BrainIr::FetchValueFrom(factor, offset) => {
					self.fetch_value_from(*factor, *offset)?;
				}
				BrainIr::ReplaceValueFrom(factor, offset) => {
					self.replace_value_from(*factor, *offset)?;
				}
				BrainIr::ScaleValue(factor) => self.scale_value(*factor)?,
				BrainIr::MemSet { value, range } => self.mem_set(*value, range.clone())?,
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
	#[allow(dead_code)]
	pub getchar: FunctionValue<'ctx>,
	pub putchar: FunctionValue<'ctx>,
	pub main: FunctionValue<'ctx>,
	pub lifetime_start: FunctionValue<'ctx>,
	pub lifetime_end: FunctionValue<'ctx>,
}

impl<'ctx> Functions<'ctx> {
	fn new(context: &'ctx Context, module: &Module<'ctx>) -> Result<Self, LlvmAssemblyError> {
		let ptr_type = context.default_ptr_type();
		let void_type = context.void_type();
		let i32_type = context.i32_type();

		let getchar_ty = void_type.fn_type(&[ptr_type.into()], false);
		let getchar = module.add_function("getchar", getchar_ty, Some(Linkage::External));

		let putchar_ty = void_type.fn_type(&[i32_type.into()], false);
		let putchar = module.add_function("putchar", putchar_ty, Some(Linkage::External));

		let main_ty = void_type.fn_type(&[], false);
		let main = module.add_function("main", main_ty, Some(Linkage::External));

		let lifetime_start_intrinsic = Intrinsic::find("llvm.lifetime.start")
			.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.start"))?;
		let lifetime_end_intrinsic = Intrinsic::find("llvm.lifetime.end")
			.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.end"))?;

		let (lifetime_start, lifetime_end) = {
			let context = module.get_context();
			let ptr_type = context.default_ptr_type();

			let lifetime_start = lifetime_start_intrinsic
				.get_declaration(module, &[ptr_type.into()])
				.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.start"))?;

			let lifetime_end = lifetime_end_intrinsic
				.get_declaration(module, &[ptr_type.into()])
				.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.end"))?;

			(lifetime_start, lifetime_end)
		};

		let this = Self {
			getchar,
			putchar,
			main,
			lifetime_start,
			lifetime_end,
		};

		Ok(this.setup(context))
	}

	fn setup(self, context: &'ctx Context) -> Self {
		self.setup_common_attributes(context)
			.setup_getchar_attributes(context)
			.setup_putchar_attributes(context)
	}

	fn setup_common_attributes(self, context: &'ctx Context) -> Self {
		let noundef_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("noundef"), 0);

		self.putchar
			.add_attribute(AttributeLoc::Param(0), noundef_attr);
		self.getchar
			.add_attribute(AttributeLoc::Param(0), noundef_attr);

		let nofree_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nofree"), 0);
		let nonlazybind_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nonlazybind"), 0);
		let uwtable_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("uwtable"), 2);
		let nocallback_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nocallback"), 0);
		let norecurse_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("norecurse"), 0);
		let willreturn_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("willreturn"), 0);
		let nosync_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nosync"), 0);

		for attribute in [
			nofree_attr,
			nonlazybind_attr,
			uwtable_attr,
			nocallback_attr,
			norecurse_attr,
			willreturn_attr,
			nosync_attr,
		] {
			self.getchar
				.add_attribute(AttributeLoc::Function, attribute);
			self.putchar
				.add_attribute(AttributeLoc::Function, attribute);
		}

		self
	}

	fn setup_getchar_attributes(self, context: &'ctx Context) -> Self {
		let memory_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("memory"), 2);

		self.getchar
			.add_attribute(AttributeLoc::Function, memory_attr);

		let writeonly_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("writeonly"), 0);
		let nocapture_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nocapture"), 0);
		let noalias_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("noalias"), 0);
		let nofree_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nofree"), 0);
		let nonnull_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nonnull"), 0);
		let dead_on_unwind_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("dead_on_unwind"), 0);

		for attribute in [
			writeonly_attr,
			nocapture_attr,
			noalias_attr,
			nofree_attr,
			nonnull_attr,
			dead_on_unwind_attr,
		] {
			self.getchar
				.add_attribute(AttributeLoc::Param(0), attribute);
		}

		self
	}

	fn setup_putchar_attributes(self, context: &'ctx Context) -> Self {
		let zeroext_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("zeroext"), 0);

		for attribute in std::iter::once(zeroext_attr) {
			self.putchar
				.add_attribute(AttributeLoc::Param(0), attribute);
		}

		self
	}
}
