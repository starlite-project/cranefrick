use std::iter;

use inkwell::{
	attributes::{Attribute, AttributeLoc},
	context::Context,
	intrinsics::Intrinsic,
	module::{Linkage, Module},
	types::BasicTypeEnum,
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

		let getchar_ty = i32_type.fn_type(&[], false);
		let getchar = module.add_function("getchar", getchar_ty, Some(Linkage::External));

		let putchar_ty = void_type.fn_type(&[i32_type.into()], false);
		let putchar = module.add_function("putchar", putchar_ty, Some(Linkage::External));

		let main_ty = void_type.fn_type(&[], false);
		let main = module.add_function("main", main_ty, None);

		let lifetime = {
			let lifetime_start = get_intrinsic_function_from_name(
				"llvm.lifetime.start",
				module,
				&[ptr_type.into()],
			)?;
			let lifetime_end =
				get_intrinsic_function_from_name("llvm.lifetime.end", module, &[ptr_type.into()])?;

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
		let noundef_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("noundef"), 0);

		for attribute in [zeroext_attr, noundef_attr] {
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

fn get_intrinsic_function_from_name<'ctx>(
	name: &'static str,
	module: &Module<'ctx>,
	types: &[BasicTypeEnum<'ctx>],
) -> Result<FunctionValue<'ctx>, LlvmAssemblyError> {
	let intrinsic =
		Intrinsic::find(name).ok_or_else(|| LlvmAssemblyError::intrinsic_not_found(name))?;

	let declaration = intrinsic
		.get_declaration(module, types)
		.ok_or_else(|| LlvmAssemblyError::invalid_intrinsic_declaration(name))?;

	Ok(declaration)
}
