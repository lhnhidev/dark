//! Native, nonblocking change notifications. No timer-based filesystem scans.
use std::io;
use std::path::Path;
#[cfg(any(windows, target_os = "linux"))]
use std::path::PathBuf;
#[cfg(any(windows, target_os = "linux"))]
use std::process::{Command, Stdio};

#[cfg(target_os = "linux")]
#[path = "watch/linux.rs"]
mod linux;
#[cfg(windows)]
#[path = "watch/windows.rs"]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::Watcher;
#[cfg(windows)]
pub use windows::Watcher;

#[cfg(not(any(windows, target_os = "linux")))]
pub struct Watcher;
#[cfg(not(any(windows, target_os = "linux")))]
impl Watcher {
    pub fn new(_: &Path) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Native file notifications are not available on this platform",
        ))
    }
    pub fn take_changed(&mut self) -> io::Result<bool> {
        Ok(false)
    }
}

#[cfg(any(windows, target_os = "linux"))]
fn git_dir(root: &Path) -> io::Result<PathBuf> {
    git_path(root, "--absolute-git-dir")
}

#[cfg(any(windows, target_os = "linux"))]
fn git_common_dir(root: &Path) -> io::Result<PathBuf> {
    git_path(root, "--git-common-dir")
}

#[cfg(any(windows, target_os = "linux"))]
fn git_path(root: &Path, argument: &str) -> io::Result<PathBuf> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--path-format=absolute", argument])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            "Cannot locate Git metadata for file notifications",
        ));
    }
    let bytes = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
    }
    #[cfg(not(unix))]
    {
        Ok(PathBuf::from(std::str::from_utf8(bytes).map_err(|_| {
            io::Error::other("Git directory path is not valid Unicode")
        })?))
    }
}

#[cfg(any(windows, target_os = "linux"))]
fn worktree_path(path: &Path) -> bool {
    let named = |part: &std::ffi::OsStr, name: &str| {
        #[cfg(windows)]
        {
            part.to_str()
                .is_some_and(|part| part.eq_ignore_ascii_case(name))
        }
        #[cfg(not(windows))]
        {
            part == name
        }
    };
    path.components()
        .next()
        .is_none_or(|part| !named(part.as_os_str(), ".dark"))
        && !path
            .components()
            .any(|part| named(part.as_os_str(), ".git"))
}

#[cfg(any(windows, target_os = "linux"))]
fn metadata_path(path: &Path) -> bool {
    path == Path::new("HEAD")
        || path == Path::new("index")
        || path == Path::new("packed-refs")
        || path.starts_with("refs/heads")
        || path == Path::new("refs")
        || path == Path::new("info")
        || path == Path::new("info/exclude")
}
