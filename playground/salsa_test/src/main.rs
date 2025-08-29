use std::{
	path::PathBuf,
	sync::{Arc, Mutex},
	time::Duration,
};

use color_eyre::{
	Report, Result,
	eyre::{Context, eyre},
};
use crossbeam_channel::{Sender, unbounded};
use dashmap::{DashMap, mapref::entry::Entry};
use notify_debouncer_mini::{
	DebounceEventResult, Debouncer, new_debouncer,
	notify::{RecommendedWatcher, RecursiveMode},
};
use salsa::{Accumulator, Setter, Storage};

fn main() -> Result<()> {
	color_eyre::install()?;

	let (tx, rx) = unbounded();
	let mut db = LazyInputDatabase::new(tx);

	let initial_file_path = std::env::args_os()
		.nth(1)
		.ok_or_else(|| eyre!("Usage: ./lazy-input <input-file>"))?;

	let initial = db.input(initial_file_path.into())?;

	loop {
		let sum = compile(&db, initial);
		let diagnostics = compile::accumulated::<Diagnostic>(&db, initial);
		if diagnostics.is_empty() {
			println!("Sum is: {sum}");
		} else {
			for diagnostic in diagnostics {
				println!("{}", diagnostic.0);
			}
		}

		for log in db.logs.lock().unwrap().drain(..) {
			eprintln!("{log}");
		}

		for event in rx.recv()?.unwrap() {
			let path = event.path.canonicalize().wrap_err_with(|| {
				format!("failed to canonicalize path {}", event.path.display())
			})?;

			let file = match db.files.get(&path) {
				Some(file) => *file,
				None => continue,
			};

			let contents = std::fs::read_to_string(path)
				.wrap_err_with(|| format!("failed to read file {}", event.path.display()))?;

			file.set_contents(&mut db).to(contents);
		}
	}
}

#[salsa::input]
struct File {
	path: PathBuf,
	#[returns(ref)]
	contents: String,
}

#[salsa::db]
trait Db: salsa::Database {
	fn input(&self, path: PathBuf) -> Result<File>;
}

#[salsa::db]
#[derive(Clone)]
struct LazyInputDatabase {
	storage: Storage<Self>,
	logs: Arc<Mutex<Vec<String>>>,
	files: DashMap<PathBuf, File>,
	file_watcher: Arc<Mutex<Debouncer<RecommendedWatcher>>>,
}

impl LazyInputDatabase {
	fn new(tx: Sender<DebounceEventResult>) -> Self {
		let logs = Arc::<Mutex<Vec<String>>>::default();
		Self {
			storage: Storage::new(Some(Box::new({
				let logs = logs.clone();
				move |event| {
					logs.lock().unwrap().push(format!("{event:?}"));
				}
			}))),
			logs,
			files: DashMap::new(),
			file_watcher: Arc::new(Mutex::new(
				new_debouncer(Duration::from_secs(1), tx).unwrap(),
			)),
		}
	}
}

#[salsa::db]
impl salsa::Database for LazyInputDatabase {}

#[salsa::db]
impl Db for LazyInputDatabase {
	fn input(&self, path: PathBuf) -> Result<File> {
		let path = path
			.canonicalize()
			.wrap_err_with(|| format!("failed to read {}", path.display()))?;

		Ok(match self.files.entry(path.clone()) {
			Entry::Occupied(entry) => *entry.get(),
			Entry::Vacant(entry) => {
				let watcher = &mut *self.file_watcher.lock().unwrap();
				watcher
					.watcher()
					.watch(&path, RecursiveMode::NonRecursive)
					.unwrap();

				let contents = std::fs::read_to_string(&path)
					.wrap_err_with(|| format!("failed to read {}", path.display()))?;

				*entry.insert(File::new(self, path, contents))
			}
		})
	}
}

#[salsa::accumulator]
struct Diagnostic(String);

impl Diagnostic {
	fn push_error(db: &dyn Db, file: File, error: Report) {
		Self(format!(
			"Error in file {}: {:?}\n",
			file.path(db)
				.file_name()
				.unwrap_or_else(|| "<unknown>".as_ref())
				.to_string_lossy(),
			error
		))
		.accumulate(db);
	}
}

#[salsa::tracked]
struct ParsedFile<'db> {
	value: u32,
	#[returns(ref)]
	links: Vec<ParsedFile<'db>>,
}

#[salsa::tracked]
fn compile(db: &dyn Db, input: File) -> u32 {
	let parsed = parse(db, input);
	sum(db, parsed)
}

#[salsa::tracked]
fn parse(db: &dyn Db, input: File) -> ParsedFile<'_> {
	let mut lines = input.contents(db).lines();
	let value = match lines.next().map(|line| (line.parse::<u32>(), line)) {
		Some((Ok(num), ..)) => num,
		Some((Err(e), line)) => {
			Diagnostic::push_error(
				db,
				input,
				Report::new(e).wrap_err(format!(
					"First line ({line}) could not be parsed as an integer"
				)),
			);
			0
		}
		None => {
			Diagnostic::push_error(db, input, eyre!("File must contain an integer"));
			0
		}
	};

	let links = lines
		.filter_map(|path| {
			let relative_path = match path.parse::<PathBuf>() {
				Ok(path) => path,
				Err(err) => {
					Diagnostic::push_error(
						db,
						input,
						Report::new(err).wrap_err(format!("Failed to parse path: {path}")),
					);
					return None;
				}
			};

			let link_path = input.path(db).parent()?.join(relative_path);
			match db.input(link_path) {
				Ok(file) => Some(parse(db, file)),
				Err(err) => {
					Diagnostic::push_error(db, input, err);
					None
				}
			}
		})
		.collect();

	ParsedFile::new(db, value, links)
}

#[salsa::tracked]
fn sum<'db>(db: &'db dyn Db, input: ParsedFile<'db>) -> u32 {
	input.value(db)
		+ input
			.links(db)
			.iter()
			.map(|&file| sum(db, file))
			.sum::<u32>()
}
