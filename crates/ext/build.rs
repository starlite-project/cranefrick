fn main() {
	cc::Build::new()
		.cpp(true)
		.warnings(false)
		.files(["./include/llvm.cpp"])
		.includes(["/usr/include/llvm-21", "/usr/include/llvm-c-21"])
		.compile("llvm_ext");

	build_rs::output::rerun_if_changed("include/llvm.cpp");
}
