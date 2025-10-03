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
	pub puts: FunctionValue<'ctx>,
	pub main: FunctionValue<'ctx>,
	pub lifetime: IntrinsicFunctionSet<'ctx>,
	pub assume: FunctionValue<'ctx>,
	pub eh_personality: FunctionValue<'ctx>,
}

impl<'ctx> AssemblerFunctions<'ctx> {
	pub fn new(context: &'ctx Context, module: &Module<'ctx>) -> Result<Self, LlvmAssemblyError> {
		let void_type = context.void_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let ptr_type = context.default_ptr_type();

		let getchar_ty = i32_type.fn_type(&[], false);
		let getchar = module.add_function("getchar", getchar_ty, Some(Linkage::External));

		let putchar_ty = i32_type.fn_type(&[i32_type.into()], false);
		let putchar = module.add_function("rust_putchar", putchar_ty, Some(Linkage::External));

		let main_ty = void_type.fn_type(&[], false);
		let main = module.add_function("main", main_ty, None);

		let puts_ty = i32_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
		let puts = module.add_function("puts", puts_ty, Some(Linkage::Private));

		let eh_personality_ty = i32_type.fn_type(
			&[
				i32_type.into(),
				i32_type.into(),
				i64_type.into(),
				ptr_type.into(),
				ptr_type.into(),
			],
			false,
		);
		let eh_personality =
			module.add_function("eh_personality", eh_personality_ty, Some(Linkage::External));

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

		let assume = get_intrinsic_function_from_name("llvm.assume", module, &[])?;

		let this = Self {
			getchar,
			putchar,
			puts,
			main,
			lifetime,
			assume,
			eh_personality,
		};

		Ok(this.setup(context))
	}

	fn setup(self, context: &'ctx Context) -> Self {
		self.main.set_personality_function(self.eh_personality);
		self.puts.set_personality_function(self.eh_personality);

		self.setup_common_attributes(context)
			.setup_getchar_attributes(context)
			.setup_put_attributes(context)
			.setup_putchar_attributes(context)
			.setup_puts_attributes(context)
			.setup_eh_personality_attributes(context)
	}

	fn setup_common_attributes(self, context: &'ctx Context) -> Self {
		let nofree_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nofree"), 0);
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
			nocallback_attr,
			norecurse_attr,
			willreturn_attr,
			nosync_attr,
		] {
			self.getchar
				.add_attribute(AttributeLoc::Function, attribute);
			self.putchar
				.add_attribute(AttributeLoc::Function, attribute);
			self.puts.add_attribute(AttributeLoc::Function, attribute);
		}

		self
	}

	fn setup_getchar_attributes(self, context: &'ctx Context) -> Self {
		let memory_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("memory"), 4);

		for attribute in iter::once(memory_attr) {
			self.getchar
				.add_attribute(AttributeLoc::Function, attribute);
		}

		let zeroext_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("zeroext"), 0);

		for attribute in iter::once(zeroext_attr) {
			self.getchar.add_attribute(AttributeLoc::Return, attribute);
		}

		self
	}

	fn setup_put_attributes(self, context: &'ctx Context) -> Self {
		let memory_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("memory"), 9);
		let uwtable_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("uwtable"), 1);

		for attribute in [memory_attr, uwtable_attr] {
			self.putchar
				.add_attribute(AttributeLoc::Function, attribute);
			self.puts.add_attribute(AttributeLoc::Function, attribute);
		}

		self
	}

	fn setup_puts_attributes(self, context: &'ctx Context) -> Self {
		let noalias_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("noalias"), 0);
		let nofree_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nofree"), 0);
		let nonnull_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nonnull"), 0);
		let readonly_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("readonly"), 0);
		let align_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("align"), 1);

		for attribute in [
			noalias_attr,
			nofree_attr,
			nonnull_attr,
			readonly_attr,
			align_attr,
		] {
			self.puts.add_attribute(AttributeLoc::Param(0), attribute);
		}

		self
	}

	fn setup_putchar_attributes(self, context: &'ctx Context) -> Self {
		let zeroext_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("zeroext"), 0);
		let noundef_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("noundef"), 0);
		let returned_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("returned"), 0);

		for attribute in [zeroext_attr, noundef_attr, returned_attr] {
			self.putchar
				.add_attribute(AttributeLoc::Param(0), attribute);
		}

		self
	}

	fn setup_eh_personality_attributes(self, context: &'ctx Context) -> Self {
		let nounwind_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nounwind"), 0);
		let nonlazybind_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nonlazybind"), 0);

		for attribute in [nounwind_attr, nonlazybind_attr] {
			self.eh_personality
				.add_attribute(AttributeLoc::Function, attribute);
		}

		let noundef_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("noundef"), 0);

		for attribute in iter::once(noundef_attr) {
			for i in 0..5 {
				self.eh_personality
					.add_attribute(AttributeLoc::Param(i), attribute);
			}
			self.eh_personality
				.add_attribute(AttributeLoc::Return, attribute);
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

#[tracing::instrument(skip(module, types), level = tracing::Level::DEBUG)]
fn get_intrinsic_function_from_name<'ctx>(
	name: &'static str,
	module: &Module<'ctx>,
	types: &[BasicTypeEnum<'ctx>],
) -> Result<FunctionValue<'ctx>, LlvmAssemblyError> {
	let intrinsic =
		Intrinsic::find(name).ok_or_else(|| LlvmAssemblyError::intrinsic_not_found(name))?;

	tracing::debug!(?intrinsic);

	let declaration = intrinsic
		.get_declaration(module, types)
		.ok_or_else(|| LlvmAssemblyError::invalid_intrinsic_declaration(name))?;

	tracing::debug!(?declaration);

	Ok(declaration)
}
