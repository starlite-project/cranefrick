use std::iter;

use inkwell::{
	attributes::{Attribute, AttributeLoc},
	context::Context,
	intrinsics::Intrinsic,
	module::{Linkage, Module},
	values::FunctionValue,
};

use crate::{ContextExt as _, LlvmAssemblyError};

#[derive(Debug, Clone, Copy)]
pub struct AssemblerFunctions<'ctx> {
	pub getchar: FunctionValue<'ctx>,
	pub putchar: FunctionValue<'ctx>,
	pub main: FunctionValue<'ctx>,
	pub lifetime: IntrinsicFunctionSet<'ctx>,
}

impl<'ctx> AssemblerFunctions<'ctx> {
	pub fn new(context: &'ctx Context, module: &Module<'ctx>) -> Result<Self, LlvmAssemblyError> {
		let ptr_type = context.default_ptr_type();
		let void_type = context.void_type();
		let i32_type = context.i32_type();

		let getchar_ty = void_type.fn_type(&[ptr_type.into()], false);
		let getchar = module.add_function("getchar", getchar_ty, Some(Linkage::External));

		let putchar_ty = void_type.fn_type(&[i32_type.into()], false);
		let putchar = module.add_function("putchar", putchar_ty, Some(Linkage::External));

		let main_ty = void_type.fn_type(&[], false);
		let main = module.add_function("main", main_ty, None);

		let lifetime_start_intrinsic = Intrinsic::find("llvm.lifetime.start")
			.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.start"))?;
		let lifetime_end_intrinsic = Intrinsic::find("llvm.lifetime.end")
			.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.end"))?;

		let lifetime = {
			let lifetime_start = lifetime_start_intrinsic
				.get_declaration(module, &[ptr_type.into()])
				.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.start"))?;
			let lifetime_end = lifetime_end_intrinsic
				.get_declaration(module, &[ptr_type.into()])
				.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.end"))?;

			IntrinsicFunctionSet::new(lifetime_start, lifetime_end)
		};

		let this = Self {
			getchar,
			putchar,
			main,
			lifetime,
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

		for attribute in iter::once(noundef_attr) {
			self.putchar
				.add_attribute(AttributeLoc::Param(0), attribute);
			self.getchar
				.add_attribute(AttributeLoc::Param(0), attribute);
		}

		let nofree_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nofree"), 0);
		let nonlazybind_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nonlazybind"), 0);
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
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("memory"), 6);

		for attribute in iter::once(memory_attr) {
			self.getchar
				.add_attribute(AttributeLoc::Function, attribute);
		}

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
		let sret_attr = context.create_type_attribute(
			Attribute::get_named_enum_kind_id("sret"),
			context.i8_type().into(),
		);

		for attribute in [
			writeonly_attr,
			nocapture_attr,
			noalias_attr,
			nofree_attr,
			nonnull_attr,
			dead_on_unwind_attr,
			sret_attr,
		] {
			self.getchar
				.add_attribute(AttributeLoc::Param(0), attribute);
		}

		self
	}

	fn setup_putchar_attributes(self, context: &'ctx Context) -> Self {
		let memory_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("memory"), 9);

		for attribute in iter::once(memory_attr) {
			self.putchar
				.add_attribute(AttributeLoc::Function, attribute);
		}

		let zeroext_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("zeroext"), 0);

		for attribute in iter::once(zeroext_attr) {
			self.putchar
				.add_attribute(AttributeLoc::Param(0), attribute);
		}

		self
	}
}

#[derive(Debug, Clone, Copy)]
pub struct IntrinsicFunctionSet<'ctx> {
	pub start: FunctionValue<'ctx>,
	pub end: FunctionValue<'ctx>,
}

impl<'ctx> IntrinsicFunctionSet<'ctx> {
	const fn new(start: FunctionValue<'ctx>, end: FunctionValue<'ctx>) -> Self {
		Self { start, end }
	}
}
