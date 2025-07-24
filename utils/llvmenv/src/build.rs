use std::{
	env, fs,
	io::{self, prelude::*},
	path::{Path, PathBuf},
	process::Command,
	sync::LazyLock,
};

use glob::glob;
use regex::Regex;
use semver::Version;
use tracing::info;

use super::{CommandExt, Error, FileIoConvert, Result, config_dir, data_dir};

const LLVMENV_FILE_NAME: &str = ".llvmenv";

#[derive(Debug)]
pub struct Build {
	name: String,
	prefix: PathBuf,
	env_path: Option<PathBuf>,
}

impl Build {
	fn system() -> Self {
		Self {
			name: "system".to_owned(),
			prefix: PathBuf::from("/usr"),
			env_path: None,
		}
	}

	#[must_use]
	pub fn from_path(path: &Path) -> Self {
		let name = path.file_name().and_then(|s| s.to_str()).unwrap();
		Self {
			name: name.to_owned(),
			prefix: path.to_owned(),
			env_path: None,
		}
	}

	pub fn from_name(name: &str) -> Result<Self> {
		if matches!(name, "system") {
			Ok(Self::system())
		} else {
			Ok(Self {
				name: name.to_owned(),
				prefix: data_dir()?.join(name),
				env_path: None,
			})
		}
	}

	#[must_use]
	pub fn exists(&self) -> bool {
		self.prefix.is_dir()
	}

	#[must_use]
	pub const fn name(&self) -> &str {
		self.name.as_str()
	}

	#[must_use]
	pub fn prefix(&self) -> &Path {
		self.prefix.as_path()
	}

	#[must_use]
	pub fn env_path(&self) -> Option<&Path> {
		self.env_path.as_deref()
	}

	pub fn set_global(&self) -> Result<()> {
		self.set_local(&config_dir()?)
	}

	pub fn set_local(&self, path: &Path) -> Result<()> {
		let env = path.join(LLVMENV_FILE_NAME);
		let mut f = fs::File::create(&env).with(&env)?;
		write!(f, "{}", self.name).with(env)?;
		info!(path = %path.display(), "writing settings");

		Ok(())
	}

	pub fn archive(&self, verbose: bool) -> Result<()> {
		let filename = format!("{}.tar.xz", self.name);
		Command::new("tar")
			.arg(if verbose { "cvf" } else { "cf" })
			.arg(&filename)
			.arg("--use-compress-prog=pixz")
			.arg(&self.name)
			.current_dir(data_dir()?)
			.check_run()?;

		println!("{}", data_dir()?.join(filename).display());
		Ok(())
	}

	pub fn version(&self) -> Result<Version> {
		let (stdout, ..) = Command::new(self.prefix().join("bin/llvm-config"))
			.arg("--version")
			.check_output()?;

		parse_version(&stdout)
	}
}

pub fn builds() -> Result<Vec<Build>> {
	let mut bs = local_builds()?;
	bs.sort_by_key(|b| b.name().to_owned());
	bs.insert(0, Build::system());
	Ok(bs)
}

pub fn seek_build() -> Result<Build> {
	let mut path = env::current_dir().unwrap();
	loop {
		if let Some(mut build) = load_local_env(&path)? {
			build.env_path = Some(path.join(LLVMENV_FILE_NAME));
			return Ok(build);
		}

		path = match path.parent() {
			Some(path) => path.to_owned(),
			None => break,
		}
	}

	if let Some(mut build) = load_global_env()? {
		build.env_path = Some(config_dir()?.join(LLVMENV_FILE_NAME));
		return Ok(build);
	}

	Ok(Build::system())
}

pub fn expand(archive: &Path, verbose: bool) -> Result<()> {
	if !archive.exists() {
		return Err(io::Error::new(
			io::ErrorKind::NotFound,
			"archive does not exist",
		))
		.with(archive);
	}

	Command::new("tar")
		.arg(if verbose { "xvf" } else { "xf" })
		.arg(archive)
		.current_dir(data_dir()?)
		.check_run()?;
	Ok(())
}

static VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+\.\d+\.\d+").unwrap());

fn parse_version(version: &str) -> Result<Version> {
	let captures = VERSION_REGEX
		.captures(version)
		.ok_or_else(|| Error::invalid_version(version.to_owned()))?;

	Version::parse(&captures[0]).map_err(|_| Error::invalid_version(version.to_owned()))
}

fn local_builds() -> Result<Vec<Build>> {
	Ok(glob(data_dir()?.join("*/bin").to_str().unwrap())
		.unwrap()
		.filter_map(|path| {
			if let Ok(path) = path {
				path.parent().map(Build::from_path)
			} else {
				None
			}
		})
		.collect())
}

fn load_local_env(path: &Path) -> Result<Option<Build>> {
	let cand = path.join(LLVMENV_FILE_NAME);
	if !cand.exists() {
		return Ok(None);
	}

	let mut f = fs::File::open(&cand).with(&cand)?;
	let mut s = String::new();
	f.read_to_string(&mut s).with(&cand)?;
	let name = s.trim();
	let mut build = Build::from_name(name)?;
	if build.exists() {
		build.env_path = Some(path.to_owned());
		Ok(Some(build))
	} else {
		Ok(None)
	}
}

fn load_global_env() -> Result<Option<Build>> {
	load_local_env(&config_dir()?)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_version_works() -> Result<()> {
		let version =
			"clang version 6.0.1-svn331815-1~exp1~20180510084719.80 (branches/release_60)";
		assert_eq!(parse_version(version)?, Version::new(6, 0, 1));

		let version = "clang version 10.0.0 \
            (https://github.com/llvm-mirror/clang 65acf43270ea2894dffa0d0b292b92402f80c8cb)";
		assert_eq!(parse_version(version)?, Version::new(10, 0, 0));

		let version = "123+456y0";
		assert!(matches!(
			parse_version(version).unwrap_err(),
			Error::InvalidVersion { .. }
		));
		assert_eq!(
			parse_version("foo 123.456.789 bar")?,
			Version::new(123, 456, 789)
		);

		Ok(())
	}
}
