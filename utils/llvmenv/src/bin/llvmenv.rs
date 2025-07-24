use std::{env, path::PathBuf, process::exit};

use clap::Parser;
use color_eyre::Result;
use llvmenv::{
	Build, BuildType, builds, expand, init_config, load_entries, load_entry, seek_build,
};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

fn main() -> Result<()> {
	install_tracing();
	color_eyre::install()?;

	let args = match LlvmEnvCommand::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};

	match args {
		LlvmEnvCommand::Init => init_config()?,
		LlvmEnvCommand::Builds => {
			let builds = builds()?;
			let max = builds.iter().map(|b| b.name().len()).max().unwrap();
			for b in &builds {
				println!(
					"{name:<width$}: {prefix}",
					name = b.name(),
					prefix = b.prefix().display(),
					width = max
				);
			}
		}
		LlvmEnvCommand::Entries => {
			if let Ok(entries) = load_entries() {
				for entry in entries {
					println!("{}", entry.name());
				}
			} else {
				panic!("no entries. Please define entries in $XDG_CONFIG_HOME/llvmenv/entry.toml");
			}
		}
		LlvmEnvCommand::BuildEntry {
			name,
			update,
			clean,
			builder,
			discard,
			nproc,
			build_type,
		} => {
			let mut entry = load_entry(&name)?;
			let nproc = nproc.unwrap_or_else(num_cpus::get);
			if let Some(builder) = builder {
				entry.set_builder(&builder)?;
			}

			if let Some(build_type) = build_type {
				entry.set_build_type(build_type)?;
			}

			if discard {
				entry.clean_cache_dir()?;
			}

			entry.checkout()?;

			if update {
				entry.update()?;
			}

			if clean {
				entry.clean_build_dir()?;
			}

			entry.build(nproc)?;
		}
		LlvmEnvCommand::Current { verbose } => {
			let build = seek_build()?;
			println!("{}", build.name());
			if verbose && let Some(env) = build.env_path() {
				eprintln!("set by {}", env.display());
			}
		}
		LlvmEnvCommand::Prefix { verbose } => {
			let build = seek_build()?;
			println!("{}", build.prefix().display());
			if verbose && let Some(env) = build.env_path() {
				eprintln!("set by {}", env.display());
			}
		}
		LlvmEnvCommand::Version {
			name,
			major,
			minor,
			patch,
		} => {
			let build = if let Some(name) = name {
				get_existing_build(&name)?
			} else {
				seek_build()?
			};

			let version = build.version()?;
			if major || minor || patch {
				if major {
					print!("{}", version.major);
				}
				if minor {
					print!("{}", version.minor);
				}
				if patch {
					print!("{}", version.patch);
				}
				println!();
			} else {
				println!("{}.{}.{}", version.major, version.minor, version.patch);
			}
		}
		LlvmEnvCommand::Global { name } => {
			let build = get_existing_build(&name)?;
			build.set_global()?;
		}
		LlvmEnvCommand::Local { name, path } => {
			let build = get_existing_build(&name)?;
			let path = path.unwrap_or_else(|| env::current_dir().unwrap());
			build.set_local(&path)?;
		}
		LlvmEnvCommand::Archive { name, verbose } => {
			let build = get_existing_build(&name)?;
			build.archive(verbose)?;
		}
		LlvmEnvCommand::Expand { path, verbose } => {
			expand(&path, verbose)?;
		}
	}

	Ok(())
}

fn install_tracing() {
	let filter_layer = EnvFilter::try_from_default_env()
		.or_else(|_| EnvFilter::try_new("info"))
		.unwrap();

	let fmt_layer = fmt::layer()
		.with_target(false)
		.compact()
		.with_filter(filter_layer);

	tracing_subscriber::registry().with(fmt_layer).init();
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
enum LlvmEnvCommand {
	/// Initialize llvmenv
	Init,
	/// List usable builds
	Builds,
	/// List entries to be built
	Entries,
	/// Build LLVM/Clang
	BuildEntry {
		name: String,
		#[arg(short, long)]
		update: bool,
		/// Clean build directory
		#[arg(short, long)]
		clean: bool,
		/// Overwrite cmake generator setting
		#[arg(short = 'G', long = "builder")]
		builder: Option<String>,
		/// Discard source directory for remote resources
		#[arg(short, long)]
		discard: bool,
		#[arg(short = 'j', long)]
		nproc: Option<usize>,
		#[allow(clippy::doc_markdown)]
		/// Overwrite cmake build type (Debug, Release, RelWithDebInfo, or MinSizeRelease)
		#[arg(short = 't', long)]
		build_type: Option<BuildType>,
	},
	/// Show the name of the current build
	Current {
		#[arg(short, long)]
		verbose: bool,
	},
	/// Show the prefix of the current build
	Prefix {
		#[arg(short, long)]
		verbose: bool,
	},
	/// Show the base version of the current build
	Version {
		#[arg(short, long)]
		name: Option<String>,
		#[arg(long)]
		major: bool,
		#[arg(long)]
		minor: bool,
		#[arg(long)]
		patch: bool,
	},
	/// Set the (global) build to use
	Global { name: String },
	/// Set the (local) build to use
	Local {
		name: String,
		#[arg(short, long)]
		path: Option<PathBuf>,
	},
	/// Archive build into *.tar.xz (requires pixz)
	Archive {
		name: String,
		#[arg(short, long)]
		verbose: bool,
	},
	/// Expand an archive
	Expand {
		path: PathBuf,
		#[arg(short, long)]
		verbose: bool,
	},
}

fn get_existing_build(name: &str) -> llvmenv::Result<Build> {
	let build = Build::from_name(name)?;
	if build.exists() {
		Ok(build)
	} else {
		eprintln!("build '{name}' does not exist");
		exit(1);
	}
}
