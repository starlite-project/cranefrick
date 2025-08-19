use std::{fs, path::PathBuf};

use ariadne::{Color, Label, Report, ReportKind, sources};
use chumsky::{Parser as _, error::Rich, input::Input as _};
use chumsky_test::{eval_expr, funcs_parser, lexer};
use clap::Parser;
use color_eyre::Result;

fn main() -> Result<()> {
	color_eyre::install()?;

	let args = match Args::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};

	let src = fs::read_to_string(&args.file_path)?;

	let (tokens, mut errs) = lexer().parse(src.as_str()).into_output_errors();

	let parse_errs = if let Some(tokens) = &tokens {
		let (ast, parse_errs) = funcs_parser()
			.map_with(|ast, e| (ast, e.span()))
			.parse(
				tokens
					.as_slice()
					.map((src.len()..src.len()).into(), |(t, s)| (t, s)),
			)
			.into_output_errors();

		if let Some((funcs, file_span)) = ast.filter(|_| errs.len() + parse_errs.len() == 0) {
			if let Some(main) = funcs.get("main") {
				if main.args.is_empty() {
					match eval_expr(&main.body, &funcs, &mut Vec::new()) {
						Ok(val) => println!("Return value: {val}"),
						Err(e) => errs.push(Rich::custom(e.span, e.message)),
					}
				} else {
					errs.push(Rich::custom(
						main.span,
						"The main function cannot have arguments".to_owned(),
					));
				}
			} else {
				errs.push(Rich::custom(
					file_span,
					"Programs need a main function but none was found".to_owned(),
				));
			}
		}

		parse_errs
	} else {
		Vec::new()
	};

	errs.into_iter()
		.map(|e| e.map_token(|c| c.to_string()))
		.chain(
			parse_errs
				.into_iter()
				.map(|e| e.map_token(|tok| tok.to_string())),
		)
		.for_each(|e| {
			Report::build(
				ReportKind::Error,
				(args.file_path.display().to_string(), e.span().into_range()),
			)
			.with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
			.with_message(e.to_string())
			.with_label(
				Label::new((args.file_path.display().to_string(), e.span().into_range()))
					.with_message(e.reason().to_string())
					.with_color(Color::Red),
			)
			.with_labels(e.contexts().map(|(label, span)| {
				Label::new((args.file_path.display().to_string(), span.into_range()))
					.with_message(format!("while parsing this {label}"))
					.with_color(Color::Yellow)
			}))
			.finish()
			.print(sources([(
				args.file_path.display().to_string(),
				src.clone(),
			)]))
			.unwrap();
		});

	Ok(())
}

#[derive(Debug, Parser)]
struct Args {
	file_path: PathBuf,
}
