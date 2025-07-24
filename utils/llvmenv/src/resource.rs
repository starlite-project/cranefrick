use std::{
	fs,
	io::{self, prelude::*},
	path::Path,
	process::Command,
};

use futures::{
	Stream,
	executor::{BlockingStream, block_on_stream},
};
use indicatif::{ProgressBar, ProgressStyle};
use tempfile::TempDir;
use tracing::{debug, info, warn};
use url::Url;

use super::{CommandExt, Error, FileIoConvert, Result};

struct Download<T> {
	stream: T,
	bytes: Option<bytes::Bytes>,
	bar: ProgressBar,
}

impl<T> Drop for Download<T> {
	fn drop(&mut self) {
		self.bar.finish();
	}
}

impl<T> Read for Download<T>
where
	T: Iterator<Item = reqwest::Result<bytes::Bytes>>,
{
	fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
		let mut bytes = if let Some(bytes) = self.bytes.take() {
			bytes
		} else {
			match self.stream.next() {
				Some(Ok(bytes)) => bytes,
				Some(Err(err)) => return Err(io::Error::other(err)),
				None => return Ok(0),
			}
		};

		if bytes.len() > buf.len() {
			self.bytes = Some(bytes.split_off(buf.len()));
		} else {
			buf = &mut buf[..bytes.len()];
		}

		buf.copy_from_slice(&bytes);
		self.bar.inc(bytes.len() as u64);
		Ok(bytes.len())
	}
}

#[derive(Debug, PartialEq, Eq)]
pub enum Resource {
	Svn { url: String },
	Git { url: String, branch: Option<String> },
	Tar { url: String },
}

impl Resource {
	pub fn from_url(url_str: &str) -> Result<Self> {
		if let Ok(filename) = get_filename_from_url(url_str) {
			for ext in [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.Z", ".tgz", ".taz"] {
				if filename.ends_with(ext) {
					debug!("found archive extension '{}' at the end of the URL", ext);
					return Ok(Self::Tar {
						url: url_str.to_owned(),
					});
				}
			}

			if filename.ends_with("trunk") {
				debug!("found 'trunk' at the end of the URL");
				return Ok(Self::Svn {
					url: url_str.to_owned(),
				});
			}

			if std::path::Path::new(&filename)
				.extension()
				.is_some_and(|ext| ext.eq_ignore_ascii_case("git"))
			{
				debug!("found '.git' extension");
				return Ok(Self::Git {
					url: strip_branch_from_url(url_str)?,
					branch: get_branch_from_url(url_str)?,
				});
			}
		}

		let url = Url::parse(url_str).map_err(|_| Error::InvalidUrl {
			url: url_str.to_owned(),
		})?;

		for service in ["github.com", "gitlab.com"] {
			if url.host_str() == Some(service) {
				debug!(service = service, "URL is a cloud git service");
				return Ok(Self::Git {
					url: strip_branch_from_url(url_str)?,
					branch: get_branch_from_url(url_str)?,
				});
			}
		}

		if url.host_str() == Some("llvm.org") {
			if url.path().starts_with("/svn") {
				debug!("URL is LLVM SVN repository");
				return Ok(Self::Svn {
					url: url_str.to_owned(),
				});
			}

			if url.path().starts_with("/git") {
				debug!("URL is LLVM Git repository");
				return Ok(Self::Git {
					url: strip_branch_from_url(url_str)?,
					branch: get_branch_from_url(url_str)?,
				});
			}
		}

		debug!(url = %url, "trying to access with git");
		let tmp_dir = TempDir::new().with("/tmp")?;
		Command::new("git")
			.arg("init")
			.current_dir(tmp_dir.path())
			.silent()
			.check_run()?;

		Command::new("git")
			.args(["remote", "add", "origin"])
			.arg(url_str)
			.current_dir(tmp_dir.path())
			.silent()
			.check_run()?;

		if Command::new("git")
			.arg("ls-remote")
			.current_dir(tmp_dir.path())
			.silent()
			.check_run()
			.is_ok()
		{
			debug!("git access successful");
			Ok(Self::Git {
				url: strip_branch_from_url(url_str)?,
				branch: get_branch_from_url(url_str)?,
			})
		} else {
			debug!("git access failed. Regarded as SVN repository");
			Ok(Self::Svn {
				url: url_str.to_owned(),
			})
		}
	}

	pub fn download(&self, dest: &Path) -> Result<()> {
		if !dest.exists() {
			fs::create_dir_all(dest).with(dest)?;
		}

		if !dest.is_dir() {
			return Err(io::Error::other("not a directory")).with(dest);
		}

		match self {
			Self::Svn { url } => Command::new("svn")
				.args(["co", url.as_str(), "-r", "HEAD"])
				.arg(dest)
				.check_run()?,
			Self::Git { url, branch } => {
				info!(url = url, "git clone");
				let mut git = Command::new("git");
				git.args(["clone", url.as_str(), "-q", "--depth", "1"])
					.arg(dest);

				if let Some(branch) = branch {
					git.args(["-b", branch.as_str()]);
				}

				git.check_run()?;
			}
			Self::Tar { url } => {
				info!(url = url, "tar file");
				let rt = tokio::runtime::Runtime::new()?;
				let mut bytes = rt.block_on(download(url))?;
				let xz_buf = xz2::read::XzDecoder::new(&mut bytes);
				let mut tar_buf = tar::Archive::new(xz_buf);
				let entries = tar_buf
					.entries()
					.expect("tar archive does not contain entry");

				for entry in entries {
					let mut entry = entry.expect("invalid entry");
					let path = entry.path().expect("filename is not valid utf-8");
					let mut target = dest.to_owned();
					for comp in path.components().skip(1) {
						target = target.join(comp);
					}

					if let Err(e) = entry.unpack(target) {
						if matches!(e.kind(), io::ErrorKind::AlreadyExists) {
							debug!("{e:?}");
						} else {
							warn!("{e:?}");
						}
					}
				}
			}
		}

		Ok(())
	}

	pub fn update(&self, dest: &Path) -> Result<()> {
		match self {
			Self::Svn { .. } => Command::new("svn")
				.arg("update")
				.current_dir(dest)
				.check_run()?,
			Self::Git { .. } => Command::new("git")
				.arg("pull")
				.current_dir(dest)
				.check_run()?,
			Self::Tar { .. } => {}
		}

		Ok(())
	}
}

async fn download(
	url: &str,
) -> Result<Download<BlockingStream<impl Stream<Item = reqwest::Result<bytes::Bytes>>>>> {
	let req = reqwest::get(url).await?;
	let status = req.status();
	if !status.is_success() {
		return Err(Error::Http {
			url: url.to_owned(),
			status,
		});
	}

	let content_length = req.headers()[reqwest::header::CONTENT_LENGTH]
		.to_str()
		.unwrap()
		.parse()?;
	let bar = ProgressBar::new(content_length).with_style(ProgressStyle::default_bar().template("{spinner:.green} [{elapsed_precise}] [{bar:38.cyan/blue}] {bytes}/{total_bytes} ({eta}) [{bytes_per_sec}]").unwrap().progress_chars("#>-"));

	Ok(Download {
		stream: block_on_stream(req.bytes_stream()),
		bytes: None,
		bar,
	})
}

fn get_filename_from_url(url_str: &str) -> Result<String> {
	let url = ::url::Url::parse(url_str).map_err(|_| Error::InvalidUrl {
		url: url_str.to_owned(),
	})?;
	let mut seg = url.path_segments().ok_or(Error::InvalidUrl {
		url: url_str.to_owned(),
	})?;
	let filename = seg.next_back().ok_or(Error::InvalidUrl {
		url: url_str.to_owned(),
	})?;

	Ok(filename.to_owned())
}

fn get_branch_from_url(url_str: &str) -> Result<Option<String>> {
	let url = ::url::Url::parse(url_str).map_err(|_| Error::InvalidUrl {
		url: url_str.to_owned(),
	})?;
	Ok(url.fragment().map(ToOwned::to_owned))
}

fn strip_branch_from_url(url_str: &str) -> Result<String> {
	let mut url = ::url::Url::parse(url_str).map_err(|_| Error::InvalidUrl {
		url: url_str.to_owned(),
	})?;
	url.set_fragment(None);
	Ok(url.to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_utils::init_tracing;

	#[test]
	fn git_download() -> Result<()> {
		init_tracing();

		let git = Resource::Git {
			url: "http://github.com/termoshtt/llvmenv".to_owned(),
			branch: None,
		};

		let tmp_dir = TempDir::new().with("/tmp")?;
		git.download(tmp_dir.path())?;
		let cargo_toml = tmp_dir.path().join("Cargo.toml");
		assert!(cargo_toml.exists());
		Ok(())
	}

	#[test]
	fn get_filename_from_url() -> Result<()> {
		let url = "http://releases.llvm.org/6.0.1/llvm-6.0.1.src.tar.xz";
		assert_eq!(super::get_filename_from_url(url)?, "llvm-6.0.1.src.tar.xz");

		Ok(())
	}

	#[test]
	fn with_git_branches() -> Result<()> {
		init_tracing();

		let github_mirror = "https://github.com/llvm-mirror/llvm";
		let git = Resource::from_url(github_mirror)?;
		assert_eq!(
			git,
			Resource::Git {
				url: github_mirror.to_owned(),
				branch: None
			}
		);

		assert_eq!(
			Resource::from_url("https://github.com/llvm-mirror/llvm#release_80")?,
			Resource::Git {
				url: github_mirror.to_owned(),
				branch: Some("release_80".to_owned())
			}
		);

		Ok(())
	}
}
