fn main() {
	build_rs::output::rerun_if_changed("include/llvm.cpp");

	cc::Build::new()
		.cpp(true)
		.includes(if cfg!(unix) {
			["/usr/include/llvm-c", "/usr/include/llvm"]
		} else {
			["C:\\LLVM\\include\\llvm", "C:\\LLVM\\include"]
		})
		.files(["./include/llvm.cpp"])
		.compile("llvm_ext");
}
