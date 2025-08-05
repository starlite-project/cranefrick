use std::{env, fmt::Write, fs, path::PathBuf};

use anyhow::{Context as _, Result};

fn main() -> Result<()> {
	autocfg::rerun_path("build.rs");
	autocfg::rerun_path("isle_examples");

	let out_dir = PathBuf::from(
		env::var_os("OUT_DIR").context("the OUT_DIR environment variable must be set")?,
	);

	let mut out = String::new();

	emit_tests(&mut out, "isle_examples/pass", "run_pass")?;
	emit_tests(&mut out, "isle_examples/fail", "run_fail")?;
	emit_tests(&mut out, "isle_examples/link", "run_link")?;
	emit_tests(&mut out, "isle_examples/run", "run_run")?;

	emit_tests(&mut out, "isle_examples/pass", "run_print")?;
	emit_tests(&mut out, "isle_examples/link", "run_print")?;
	emit_tests(&mut out, "isle_examples/run", "run_print")?;

	let output = out_dir.join("isle_tests.rs");
	fs::write(output, out)?;

	Ok(())
}

#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn emit_tests(out: &mut String, dir_name: &str, runner_func: &str) -> Result<()> {
	for test_file in fs::read_dir(dir_name)? {
		let test_file = test_file?
			.file_name()
			.into_string()
			.ok()
			.context("failed to convert file name to utf-8")?;

		if !test_file.ends_with(".isle") {
			continue;
		}

		let test_file_base = test_file.replace(".isle", "");

		writeln!(out, "#[test]")?;
		writeln!(out, "fn test_{runner_func}_{test_file_base}() {{")?;
		writeln!(out, "    {runner_func}(\"{dir_name}/{test_file}\");")?;
		writeln!(out, "}}")?;
	}

	Ok(())
}
