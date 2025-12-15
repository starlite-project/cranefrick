fn main() {
	build_rs::output::rerun_if_changed("include/llvm.cpp");

	cc::Build::new()
		.cpp(true)
		.includes(["/usr/include/llvm-c", "/usr/include/llvm"])
		.files(["./include/llvm.cpp"])
		.compile("llvm_ext");
}
