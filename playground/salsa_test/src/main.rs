use salsa_test::{
	compile,
	db::CalcDatabase,
	ir::{Diagnostic, SourceProgram},
};

fn main() {
	let db: CalcDatabase = CalcDatabase::default();
	let source_program = SourceProgram::new(&db, String::from("print 1 + 2 * 3 + 4"));
	compile::compile(&db, source_program);
	let diagnostics = compile::compile::accumulated::<Diagnostic>(&db, source_program);
	eprintln!("{diagnostics:?}");
}
