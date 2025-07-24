use std::{collections::HashMap, fs, path::PathBuf, process, str::FromStr, sync::LazyLock};

use itertools::Itertools as _;
use semver::{Version, VersionReq};
use serde::Deserialize;
use tracing::{info, warn};

use super::{
	CommandExt, ENTRY_TOML, Error, FileIoConvert as _, Resource, Result, cache_dir, config_dir,
	data_dir,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Tool {
	pub name: String,
	pub url: String,
	pub branch: Option<String>,
	pub relative_path: Option<String>,
}

impl Tool {
	const fn new(name: String, url: String) -> Self {
		Self {
			name,
			url,
			branch: None,
			relative_path: None,
		}
	}

	fn relative_path(&self) -> String {
		match &self.relative_path {
			Some(rel_path) => rel_path.clone(),
			None => match self.name.as_str() {
				"clang" | "lld" | "lldb" | "polly" => format!("tools/{}", self.name),
				"clang-tools-extra" => "tools/clang/tools/clang-tools-extra".into(),
				"compiler-rt" | "libcxx" | "libcxxabi" | "libunwind" | "openmp" => {
					format!("projects/{}", self.name)
				}
				_ => panic!(
					"Unknown tool. Please specify its relative path explicitly: {}",
					self.name
				),
			},
		}
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub struct EntrySetting {
	pub url: Option<String>,
	pub path: Option<String>,
	#[serde(default)]
	pub tools: Vec<Tool>,
	#[serde(default)]
	pub target: Vec<String>,
	#[serde(default)]
	pub generator: CMakeGenerator,
	#[serde(default)]
	pub build_type: BuildType,
	#[serde(default)]
	pub option: HashMap<String, String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum CMakeGenerator {
	#[default]
	Platform,
	Makefile,
	Ninja,
	VisualStudio,
	VisualStudioWin64,
}

impl CMakeGenerator {
	pub fn option(self) -> Vec<String> {
		match self {
			Self::Platform => Vec::new(),
			Self::Makefile => vec!["-G", "Unit Makefiles"],
			Self::Ninja => vec!["-G", "Ninja"],
			Self::VisualStudio => vec!["-G", "Visual Studio 15 2017"],
			Self::VisualStudioWin64 => vec!["-G", "Visual Studio 15 2017 Win64", "-Thost=x64"],
		}
		.into_iter()
		.map(ToOwned::to_owned)
		.collect()
	}

	#[must_use]
	pub fn build_option(self, nproc: usize, build_type: BuildType) -> Vec<String> {
		match self {
			Self::VisualStudioWin64 | Self::VisualStudio => {
				vec!["--config".to_owned(), format!("{build_type:?}")]
			}
			Self::Platform => Vec::new(),
			Self::Makefile | Self::Ninja => {
				vec!["--".to_owned(), "-j".to_owned(), format!("{nproc}")]
			}
		}
	}
}

impl FromStr for CMakeGenerator {
	type Err = Error;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(match s.to_ascii_lowercase().as_str() {
			"makefile" => Self::Makefile,
			"ninja" => Self::Ninja,
			"visualstudio" | "vs" => Self::VisualStudio,
			_ => {
				return Err(Error::UnsupportedGenerator {
					generator: s.to_owned(),
				});
			}
		})
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BuildType {
	Debug,
	#[default]
	Release,
	ReleaseWithDebugInfo,
	MinSizeRelease,
}

impl FromStr for BuildType {
	type Err = Error;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(match s.to_ascii_lowercase().as_str() {
			"debug" => Self::Debug,
			"release" => Self::Release,
			"relwithdebinfo" => Self::ReleaseWithDebugInfo,
			"minsizerel" => Self::MinSizeRelease,
			_ => {
				return Err(Error::UnsupportedBuildType {
					build_type: s.to_owned(),
				});
			}
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
	Remote {
		name: String,
		version: Option<Version>,
		url: String,
		tools: Vec<Tool>,
		setting: EntrySetting,
	},
	Local {
		name: String,
		version: Option<Version>,
		path: PathBuf,
		setting: EntrySetting,
	},
}

impl Entry {
	fn parse_setting(
		name: String,
		version: Option<Version>,
		setting: EntrySetting,
	) -> Result<Self> {
		if setting.path.is_some() && setting.url.is_some() {
			return Err(Error::InvalidEntry {
				name,
				message: "only path or URL is allowed".to_owned(),
			});
		}

		if let Some(path) = &setting.path {
			if !setting.tools.is_empty() {
				warn!("'tools' must be used with URL, ignored");
			}

			return Ok(Self::Local {
				name,
				version,
				path: PathBuf::from(shellexpand::full(&path).unwrap().into_owned()),
				setting,
			});
		}

		if let Some(url) = &setting.url {
			return Ok(Self::Remote {
				name,
				version,
				url: url.clone(),
				tools: setting.tools.clone(),
				setting,
			});
		}

		Err(Error::InvalidEntry {
			name,
			message: "path and URL are missing".to_owned(),
		})
	}

	#[must_use]
	pub fn official(major: u64, minor: u64, patch: u64) -> Self {
		const LLVM_8_0_1: Version = Version::new(8, 0, 1);
		const LLVM_9_0_0: Version = Version::new(9, 0, 0);

		let version = Version::new(major, minor, patch);
		let mut setting = EntrySetting::default();

		let base_url = if version <= LLVM_9_0_0 && version != LLVM_8_0_1 {
			format!("https://releases.llvm.org/{version}")
		} else {
			format!("https://github.com/llvm/llvm-project/releases/download/llvmorg-{version}")
		};

		setting.url = Some(format!("{base_url}/llvm-{version}.src.tar.xz"));
		setting.tools.extend([
			Tool::new(
				"clang".to_owned(),
				format!(
					"{base_url}/{}-{version}.src.tar.xz",
					if version > LLVM_9_0_0 { "clang" } else { "cfe" }
				),
			),
			Tool::new(
				"lld".to_owned(),
				format!("{base_url}/lld-{version}.src.tar.xz"),
			),
			Tool::new(
				"lldb".to_owned(),
				format!("{base_url}/lldb-{version}.src.tar.xz"),
			),
			Tool::new(
				"clang-tools-extra".to_owned(),
				format!("{base_url}/clang-tools-extra-{version}.src.tar.xz"),
			),
			Tool::new(
				"polly".to_owned(),
				format!("{base_url}/polly-{version}.src.tar.xz"),
			),
			Tool::new(
				"compiler-rt".to_owned(),
				format!("{base_url}/compiler-rt-{version}.src.tar.xz"),
			),
			Tool::new(
				"libcxx".to_owned(),
				format!("{base_url}/libcxx-{version}.src.tar.xz"),
			),
			Tool::new(
				"libcxxabi".to_owned(),
				format!("{base_url}/libcxxabi-{version}.src.tar.xz"),
			),
			Tool::new(
				"libunwind".to_owned(),
				format!("{base_url}/libunwind-{version}.src.tar.xz"),
			),
			Tool::new(
				"openmp".to_owned(),
				format!("{base_url}/openmp-{version}.src.tar.xz"),
			),
		]);

		let name = version.to_string();
		Self::parse_setting(name, Some(version), setting).unwrap()
	}

	const fn setting(&self) -> &EntrySetting {
		match self {
			Self::Remote { setting, .. } | Self::Local { setting, .. } => setting,
		}
	}

	const fn setting_mut(&mut self) -> &mut EntrySetting {
		match self {
			Self::Remote { setting, .. } | Self::Local { setting, .. } => setting,
		}
	}

	pub fn set_builder(&mut self, generator: &str) -> Result<()> {
		let generator = CMakeGenerator::from_str(generator)?;
		self.setting_mut().generator = generator;
		Ok(())
	}

	pub const fn set_build_type(&mut self, build_type: BuildType) -> Result<()> {
		self.setting_mut().build_type = build_type;
		Ok(())
	}

	pub fn checkout(&self) -> Result<()> {
		let Self::Remote { url, tools, .. } = self else {
			return Ok(());
		};

		let src = Resource::from_url(url)?;
		src.download(&self.src_dir()?)?;
		for tool in tools {
			let path = self.src_dir()?.join(tool.relative_path());
			let src = Resource::from_url(&tool.url)?;
			src.download(&path)?;
		}

		Ok(())
	}

	pub fn clean_cache_dir(&self) -> Result<()> {
		let path = self.src_dir()?;
		info!(path = %path.display(), "removing cache directory");
		fs::remove_dir_all(&path).with(&path)
	}

	pub fn update(&self) -> Result<()> {
		let Self::Remote { url, tools, .. } = self else {
			return Ok(());
		};

		let src = Resource::from_url(url)?;
		src.update(&self.src_dir()?)?;
		for tool in tools {
			let src = Resource::from_url(&tool.url)?;
			src.update(&self.src_dir()?.join(tool.relative_path()))?;
		}

		Ok(())
	}

	#[must_use]
	pub fn name(&self) -> &str {
		match self {
			Self::Local { name, .. } | Self::Remote { name, .. } => name,
		}
	}

	#[must_use]
	pub const fn version(&self) -> Option<&Version> {
		match self {
			Self::Remote { version, .. } | Self::Local { version, .. } => version.as_ref(),
		}
	}

	pub fn src_dir(&self) -> Result<PathBuf> {
		Ok(match self {
			Self::Remote { name, .. } => cache_dir()?.join(name),
			Self::Local { path, .. } => path.to_owned(),
		})
	}

	pub fn build_dir(&self) -> Result<PathBuf> {
		let dir = self.src_dir()?.join("build");
		if !dir.exists() {
			info!(path = %dir.display(), "creating build directory");
			fs::create_dir_all(&dir).with(&dir)?;
		}

		Ok(dir)
	}

	pub fn clean_build_dir(&self) -> Result<()> {
		let path = self.build_dir()?;
		info!(path = %path.display(), "removing build directory");
		fs::remove_dir_all(&path).with(&path)
	}

	pub fn prefix(&self) -> Result<PathBuf> {
		Ok(data_dir()?.join(self.name()))
	}

	pub fn build(&self, nproc: usize) -> Result<()> {
		self.configure()?;
		process::Command::new("cmake")
			.args([
				"--build",
				&format!("{}", self.build_dir()?.display()),
				"--target",
				"install",
			])
			.args(
				self.setting()
					.generator
					.build_option(nproc, self.setting().build_type),
			)
			.check_run()
	}

	fn configure(&self) -> Result<()> {
		let setting = self.setting();
		let mut opts = setting.generator.option();
		opts.push(self.src_dir()?.display().to_string());

		opts.push(format!(
			"-DCMAKE_INSTALL_PREFIX={}",
			data_dir()?.join(self.prefix()?).display()
		));
		opts.push(format!("-DCMAKE_BUILD_TYPE={:?}", setting.build_type));

		if which::which("ccache").is_ok() {
			opts.push("-DLLVM_CCACHE_BUILD=ON".to_owned());
		}

		if which::which("lld").is_ok() {
			opts.push("-DLLVM_ENABLE_LLD=ON".to_owned());
		}

		if !setting.target.is_empty() {
			opts.push(format!(
				"-DLLVM_TARGETS_TO_BUILD={}",
				setting.target.iter().join(";")
			));
		}

		for (k, v) in &setting.option {
			opts.push(format!("-D{k}={v}"));
		}

		process::Command::new("cmake")
			.args(&opts)
			.current_dir(self.build_dir()?)
			.check_run()
	}
}

static ENTRIES: LazyLock<[Entry; 71]> = LazyLock::new(|| {
	[
		Entry::official(20, 1, 8),
		Entry::official(20, 1, 7),
		Entry::official(20, 1, 6),
		Entry::official(20, 1, 5),
		Entry::official(20, 1, 4),
		Entry::official(20, 1, 3),
		Entry::official(20, 1, 2),
		Entry::official(20, 1, 1),
		Entry::official(20, 1, 0),
		Entry::official(19, 1, 0),
		Entry::official(18, 1, 8),
		Entry::official(18, 1, 7),
		Entry::official(18, 1, 6),
		Entry::official(18, 1, 5),
		Entry::official(18, 1, 4),
		Entry::official(18, 1, 3),
		Entry::official(18, 1, 2),
		Entry::official(18, 1, 1),
		Entry::official(18, 1, 0),
		Entry::official(17, 0, 6),
		Entry::official(17, 0, 5),
		Entry::official(17, 0, 4),
		Entry::official(17, 0, 3),
		Entry::official(17, 0, 2),
		Entry::official(17, 0, 1),
		Entry::official(17, 0, 0),
		Entry::official(16, 0, 6),
		Entry::official(16, 0, 5),
		Entry::official(16, 0, 4),
		Entry::official(16, 0, 3),
		Entry::official(16, 0, 2),
		Entry::official(16, 0, 1),
		Entry::official(16, 0, 0),
		Entry::official(15, 0, 7),
		Entry::official(15, 0, 6),
		Entry::official(15, 0, 5),
		Entry::official(15, 0, 4),
		Entry::official(15, 0, 3),
		Entry::official(15, 0, 2),
		Entry::official(15, 0, 1),
		Entry::official(15, 0, 0),
		Entry::official(14, 0, 6),
		Entry::official(14, 0, 5),
		Entry::official(14, 0, 4),
		Entry::official(14, 0, 3),
		Entry::official(14, 0, 2),
		Entry::official(14, 0, 1),
		Entry::official(14, 0, 0),
		Entry::official(13, 0, 1),
		Entry::official(13, 0, 0),
		Entry::official(12, 0, 1),
		Entry::official(12, 0, 0),
		Entry::official(11, 1, 0),
		Entry::official(11, 0, 0),
		Entry::official(10, 0, 1),
		Entry::official(10, 0, 0),
		Entry::official(9, 0, 1),
		Entry::official(8, 0, 1),
		Entry::official(9, 0, 0),
		Entry::official(8, 0, 0),
		Entry::official(7, 1, 0),
		Entry::official(7, 0, 1),
		Entry::official(7, 0, 0),
		Entry::official(6, 0, 1),
		Entry::official(6, 0, 0),
		Entry::official(5, 0, 2),
		Entry::official(5, 0, 1),
		Entry::official(4, 0, 1),
		Entry::official(4, 0, 0),
		Entry::official(3, 9, 1),
		Entry::official(3, 9, 0),
	]
});

#[must_use]
pub fn official_releases() -> &'static [Entry] {
	&*ENTRIES
}

pub fn load_entries() -> Result<Vec<Entry>> {
	let global_toml = config_dir()?.join(ENTRY_TOML);
	let mut entries = load_entry_toml(&fs::read_to_string(&global_toml).with(&global_toml)?)?;
	let official = official_releases();
	entries.extend_from_slice(official);
	Ok(entries)
}

pub fn load_entry(name: &str) -> Result<Entry> {
	let entries = load_entries()?;
	for entry in entries {
		if entry.name() == name {
			return Ok(entry);
		}

		if let Some(version) = entry.version()
			&& let Ok(req) = VersionReq::parse(name)
			&& req.matches(version)
		{
			return Ok(entry);
		}
	}

	Err(Error::InvalidEntry {
		name: name.to_owned(),
		message: "entry not found".to_owned(),
	})
}

fn load_entry_toml(toml_str: &str) -> Result<Vec<Entry>> {
	let entries = toml::from_str::<HashMap<String, EntrySetting>>(toml_str)?;

	entries
		.into_iter()
		.map(|(name, setting)| {
			Entry::parse_setting(name.clone(), Version::parse(&name).ok(), setting)
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_url() {
		let setting = EntrySetting {
			url: Some("http://llvm.org/svn/llvm-project/llvm/trunk".to_owned()),
			..Default::default()
		};

		assert!(Entry::parse_setting("url".to_owned(), None, setting).is_ok());
	}
}
