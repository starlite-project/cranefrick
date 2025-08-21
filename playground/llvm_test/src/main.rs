use color_eyre::{
	Result,
	eyre::{ContextCompat, Report},
};
use inkwell::{
	OptimizationLevel,
	context::Context,
	targets::{InitializationConfig, Target},
};

fn main() -> Result<()> {
	color_eyre::install()?;

	Target::initialize_native(&InitializationConfig::default()).map_err(Report::msg)?;

	let context = Context::create();
	let module = context.create_module("test");
	let builder = context.create_builder();

	let ft = context.f64_type();
	let fnt = ft.fn_type(&[], false);

	let f = module.add_function("test_fn", fnt, None);
	let b = context.append_basic_block(f, "entry");

	builder.position_at_end(b);

	let extf = module.add_function("sumf", ft.fn_type(&[ft.into(), ft.into()], false), None);

	let argf = ft.const_float(64.0);
	let call_site_value = builder.build_call(extf, &[argf.into(), argf.into()], "retv")?;
	let retv = call_site_value
		.try_as_basic_value()
		.left()
		.context("could not create basic value")?
		.into_float_value();

	builder.build_return(Some(&retv))?;

	let ee = module
		.create_jit_execution_engine(OptimizationLevel::Aggressive)
		.map_err(|e| Report::msg(e.to_string()))?;
	ee.add_global_mapping(&extf, sumf as usize);

	module.print_to_stderr();

	let result = unsafe { ee.run_function(f, &[]) }.as_float(&ft);

	println!("{result}");

	Ok(())
}

extern "C" fn sumf(a: f64, b: f64) -> f64 {
	a + b
}
