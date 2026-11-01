//! Synchronous child processes with disk-backed output and bounded execution.
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct Output {
    pub path: PathBuf,
    pub success: bool,
}

pub fn run(command: &mut Command, directory: &Path, limit: u64) -> io::Result<Output> {
    fs::create_dir_all(directory)?;
    reject_link(directory)?;
    let path = directory.join("stdout");
    let error = directory.join("stderr");
    reject_link(&path)?;
    reject_link(&error)?;
    command
        .stdin(Stdio::null())
        .stdout(File::create(&path)?)
        .stderr(File::create(&error)?);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if fs::metadata(&path)?.len() > limit || fs::metadata(&error)?.len() > limit {
                return Err(io::Error::other("Git output exceeds the safe disk limit"));
            }
            return Ok(Output {
                path,
                success: status.success(),
            });
        }
        if start.elapsed() > Duration::from_secs(30)
            || fs::metadata(&path)?.len() > limit
            || fs::metadata(&error)?.len() > limit
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other(
                "Git timed out or exceeded the output limit; operation stopped",
            ));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub fn reject_link(path: &Path) -> io::Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err(io::Error::other(
                "Generated output path cannot be a symlink",
            ));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(io::Error::other(
                    "Generated output path cannot be a reparse point",
                ));
            }
        }
    }
    Ok(())
}
