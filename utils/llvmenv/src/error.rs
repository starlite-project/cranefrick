use std::{
	error::Error as StdError,
	fmt::{Display, Formatter, Result as FmtResult, Write as _},
	io,
	num::ParseIntError,
	path::{Path, PathBuf},
	process,
};

#[derive(Debug)]
pub enum Error {
	FileIo {
		path: PathBuf,
		source: io::Error,
	},
	FileIoExtra {
		source: fs_extra::error::Error,
	},
	UnsupportedOs,
	UnsupportedGenerator {
		generator: String,
	},
	UnsupportedBuildType {
		build_type: String,
	},
	ConfigureAlreadyExists {
		path: PathBuf,
	},
	InvalidVersion {
		version: String,
	},
	InvalidUrl {
		url: String,
	},
	InvalidToml {
		source: toml::de::Error,
	},
	InvalidEntry {
		name: String,
		message: String,
	},
	Http {
		url: String,
		status: reqwest::StatusCode,
	},
	Io {
		source: io::Error,
	},
	ParseInt {
		source: ParseIntError,
	},
	Reqwest {
		source: reqwest::Error,
	},
	Command {
		errno: i32,
		command: String,
		stdout: Option<String>,
		stderr: Option<String>,
	},
	CommandNotFound {
		command: String,
	},
	CommandTerminatedBySignal {
		command: String,
		stdout: Option<String>,
		stderr: Option<String>,
	},
}

impl Error {
	#[must_use]
	pub const fn invalid_version(version: String) -> Self {
		Self::InvalidVersion { version }
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::FileIo { path, .. } => {
				f.write_str("io error while accessing ")?;
				Display::fmt(&path.display(), f)?;
			}
			Self::FileIoExtra { .. } => f.write_str("idk man")?,
			Self::UnsupportedOs => {
				f.write_str("unsupported OS which cannot get (config|cache|data) directory")?;
			}
			Self::UnsupportedGenerator { generator } => {
				f.write_str("unsupported cmake generator: ")?;
				f.write_str(generator)?;
			}
			Self::UnsupportedBuildType { build_type } => {
				f.write_str("unsupported cmake build type: ")?;
				f.write_str(build_type)?;
			}
			Self::ConfigureAlreadyExists { path } => {
				f.write_str("configuration file already exists at ")?;
				Display::fmt(&path.display(), f)?;
			}
			Self::InvalidVersion { version } => {
				f.write_str("failed to get LLVM version: ")?;
				f.write_str(version)?;
			}
			Self::InvalidUrl { url } => {
				f.write_str("invalid URL: ")?;
				f.write_str(url)?;
			}
			Self::InvalidToml { .. } => f.write_str("an error occurred parsing toml")?,
			Self::InvalidEntry { name, message } => {
				f.write_str("entry ")?;
				f.write_str(name)?;
				f.write_str(" is invalid: ")?;
				f.write_str(message)?;
			}
			Self::Http { url, status } => {
				f.write_str("HTTP request does not succeed with ")?;
				Display::fmt(&status, f)?;
				f.write_str(": ")?;
				f.write_str(url)?;
			}
			Self::Io { .. } => f.write_str("an io error occurred")?,
			Self::ParseInt { .. } => f.write_str("a parsing error occurred")?,
			Self::Reqwest { .. } => f.write_str("a reqwest error occurred")?,
			Self::Command { errno, command, .. } => {
				f.write_str("external command ")?;
				f.write_str(command)?;
				f.write_str(" exited with error-code(")?;
				Display::fmt(&errno, f)?;
				f.write_char(')')?;
			}
			Self::CommandNotFound { command } => {
				f.write_str("external command ")?;
				f.write_str(command)?;
				f.write_str(" not found")?;
			}
			Self::CommandTerminatedBySignal { command, .. } => {
				f.write_str("external command ")?;
				f.write_str(command)?;
				f.write_str(" has been terminated by signal")?;
			}
		}

		Ok(())
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::FileIo { source, .. } | Self::Io { source } => Some(source),
			Self::FileIoExtra { source } => Some(source),
			Self::InvalidToml { source } => Some(source),
			Self::ParseInt { source } => Some(source),
			Self::Reqwest { source } => Some(source),
			_ => None,
		}
	}
}

impl From<fs_extra::error::Error> for Error {
	fn from(value: fs_extra::error::Error) -> Self {
		Self::FileIoExtra { source: value }
	}
}

impl From<toml::de::Error> for Error {
	fn from(value: toml::de::Error) -> Self {
		Self::InvalidToml { source: value }
	}
}

impl From<io::Error> for Error {
	fn from(value: io::Error) -> Self {
		Self::Io { source: value }
	}
}

impl From<ParseIntError> for Error {
	fn from(value: ParseIntError) -> Self {
		Self::ParseInt { source: value }
	}
}

impl From<reqwest::Error> for Error {
	fn from(value: reqwest::Error) -> Self {
		Self::Reqwest { source: value }
	}
}

pub trait FileIoConvert<T> {
	fn with(self, path: impl AsRef<Path>) -> Result<T>;
}

impl<T> FileIoConvert<T> for Result<T, io::Error> {
	fn with(self, path: impl AsRef<Path>) -> Result<T> {
		self.map_err(|source| Error::FileIo {
			source,
			path: path.as_ref().to_owned(),
		})
	}
}

pub trait CommandExt {
	fn silent(&mut self) -> &mut Self;

	fn check_run(&mut self) -> Result<()>;

	fn check_output(&mut self) -> Result<(String, String)>;
}

impl CommandExt for process::Command {
	fn silent(&mut self) -> &mut Self {
		self.stdout(process::Stdio::null())
			.stderr(process::Stdio::null())
	}

	fn check_run(&mut self) -> Result<()> {
		let command = format!("{self:?}");

		let st = self.status().map_err(|_| Error::CommandNotFound {
			command: command.clone(),
		})?;

		match st.code() {
			Some(0) => Ok(()),
			Some(errno) => Err(Error::Command {
				errno,
				command,
				stderr: None,
				stdout: None,
			}),
			None => Err(Error::CommandTerminatedBySignal {
				command,
				stdout: None,
				stderr: None,
			}),
		}
	}

	fn check_output(&mut self) -> Result<(String, String)> {
		let command = format!("{self:?}");
		let output = self.output().map_err(|_| Error::CommandNotFound {
			command: command.clone(),
		})?;
		let stdout = String::from_utf8(output.stdout).expect("invalid UTF-8");
		let stderr = String::from_utf8(output.stderr).expect("invalid UTF-8");

		match output.status.code() {
			Some(0) => Ok((stdout, stderr)),
			Some(errno) => Err(Error::Command {
				errno,
				command,
				stderr: Some(stderr),
				stdout: Some(stdout),
			}),
			None => Err(Error::CommandTerminatedBySignal {
				command,
				stdout: Some(stdout),
				stderr: Some(stderr),
			}),
		}
	}
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
