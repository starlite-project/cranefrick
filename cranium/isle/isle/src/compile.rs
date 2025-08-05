use std::{
	path::{Path, PathBuf},
	sync::Arc,
};

use super::{
	ast::{self, Def},
	codegen,
	error::Errors,
	files::Files,
	overlap, sema,
};
use crate::files::TryFromFilesError;

pub fn compile(
	files: Arc<Files>,
	defs: &[ast::Def],
	options: &codegen::CodegenOptions,
) -> Result<String, Errors> {
	let mut type_env = match sema::TypeEnv::try_from_ast(defs) {
		Ok(type_env) => type_env,
		Err(e) => return Err(Errors::new(e, files)),
	};

	let term_env = match sema::TermEnv::try_from_ast(&mut type_env, defs, true) {
		Ok(term_env) => term_env,
		Err(e) => return Err(Errors::new(e, files)),
	};

	let terms = match overlap::check(&term_env) {
		Ok(terms) => terms,
		Err(e) => return Err(Errors::new(e, files)),
	};

	Ok(codegen::codegen(
		files, &type_env, &term_env, &terms, options,
	))
}

pub fn from_files<P>(
	inputs: impl IntoIterator<Item = P>,
	options: &codegen::CodegenOptions,
) -> Result<String, Errors>
where
	P: AsRef<Path>,
{
	let files = match Files::try_from_paths(inputs, &options.prefixes) {
		Ok(files) => Arc::new(files),
		Err(TryFromFilesError(path, err)) => {
			return Err(Errors::from_io(
				err,
				format!("cannot read file {}", path.display()),
			));
		}
	};

	let mut defs = Vec::new();
	for (file, src) in files.texts.iter().enumerate() {
		let lexer = match super::lexer::Lexer::new(file, src) {
			Ok(lexer) => lexer,
			Err(error) => return Err(Errors::new([error], files)),
		};

		match super::parser::parse(lexer) {
			Ok(mut ds) => defs.append(&mut ds),
			Err(e) => return Err(Errors::new([e], files)),
		}
	}

	compile(files, &defs, options)
}

pub fn create_envs(
	inputs: impl IntoIterator<Item = PathBuf>,
) -> Result<(sema::TypeEnv, sema::TermEnv, Vec<Def>), Errors> {
	let files = match Files::try_from_paths(inputs, &[]) {
		Ok(files) => files,
		Err(TryFromFilesError(path, err)) => {
			return Err(Errors::from_io(
				err,
				format!("cannot read file {}", path.display()),
			));
		}
	};
	let files = Arc::new(files);
	let mut defs = Vec::new();
	for (file, src) in files.texts.iter().enumerate() {
		let lexer = match crate::lexer::Lexer::new(file, src) {
			Ok(lexer) => lexer,
			Err(err) => return Err(Errors::new(vec![err], files)),
		};

		match crate::parser::parse(lexer) {
			Ok(mut ds) => defs.append(&mut ds),
			Err(err) => return Err(Errors::new(vec![err], files)),
		}
	}
	let mut type_env = match sema::TypeEnv::try_from_ast(&defs) {
		Ok(type_env) => type_env,
		Err(errs) => return Err(Errors::new(errs, files)),
	};
	let term_env = match sema::TermEnv::try_from_ast(&mut type_env, &defs, false) {
		Ok(term_env) => term_env,
		Err(errs) => return Err(Errors::new(errs, files)),
	};
	Ok((type_env, term_env, defs))
}
