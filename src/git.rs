use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct Repo {
    pub root: PathBuf,
    index_path: PathBuf,
    scratch: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct StatusFile {
    pub path: PathBuf,
    pub display_path: String,
    pub status: char,
    pub staged: bool,
    pub changed: bool,
    pub untracked: bool,
    pub conflict: bool,
    pub added: Option<u64>,
    pub deleted: Option<u64>,
}

pub fn error(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

pub fn path_from_bytes(bytes: &[u8]) -> io::Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
    }
    #[cfg(not(unix))]
    {
        Ok(PathBuf::from(std::str::from_utf8(bytes).map_err(|_| {
            error("Git path is not valid Unicode on this platform")
        })?))
    }
}

impl Repo {
    pub fn discover(path: &Path) -> io::Result<Self> {
        let mut command = Command::new("git");
        command
            .current_dir(path)
            .args(["rev-parse", "--show-toplevel"]);
        static DISCOVER_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let instance = DISCOVER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let scratch =
            std::env::temp_dir().join(format!("dark-discover-{}-{}", std::process::id(), instance));
        let result = crate::proc::run(&mut command, &scratch, 16384)?;
        if !result.success {
            return Err(error("Open a Git working tree to review changes"));
        }
        let bytes = fs::read(result.path)?;
        let root = path_from_bytes(
            bytes
                .strip_suffix(b"\n")
                .unwrap_or(&bytes)
                .strip_suffix(b"\r")
                .unwrap_or(bytes.strip_suffix(b"\n").unwrap_or(&bytes)),
        )?;
        let mut command = Command::new("git");
        command
            .current_dir(&root)
            .args(["rev-parse", "--absolute-git-dir"]);
        let result = crate::proc::run(&mut command, &scratch, 16384)?;
        if !result.success {
            return Err(error("Cannot locate Git metadata"));
        }
        let bytes = fs::read(result.path)?;
        let index_path = path_from_bytes(
            bytes
                .strip_suffix(b"\n")
                .unwrap_or(&bytes)
                .strip_suffix(b"\r")
                .unwrap_or(bytes.strip_suffix(b"\n").unwrap_or(&bytes)),
        )?
        .join("index");
        let _ = fs::remove_dir_all(scratch);
        let scratch = root.join(format!(".dark/process-{}-{instance}", std::process::id()));
        Ok(Self {
            root,
            index_path,
            scratch,
        })
    }

    pub fn command(&self, args: &[OsString], index: Option<&Path>) -> io::Result<PathBuf> {
        let mut command = Command::new("git");
        command
            .current_dir(&self.root)
            .args([
                "--no-pager",
                "--literal-pathspecs",
                "-c",
                "core.quotePath=false",
                "-c",
                "diff.external=",
                "-c",
                "core.fsmonitor=false",
            ])
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0");
        crate::proc::reject_link(&self.root.join(".dark"))?;
        let directory = &self.scratch;
        fs::create_dir_all(directory)?;
        crate::proc::reject_link(directory)?;
        if let Some(index) = index {
            command.env("GIT_INDEX_FILE", index);
        } else {
            // Even read-only Git commands can refresh index stat data. Use a disposable copy.
            let private = directory.join("read-index");
            crate::proc::reject_link(&private)?;
            if self.index_path.exists() {
                fs::copy(&self.index_path, &private)?;
            } else if private.exists() {
                fs::remove_file(&private)?;
            }
            command.env("GIT_INDEX_FILE", private);
        }
        let result = crate::proc::run(&mut command, directory, 64 * 1024 * 1024)?;
        let _ = fs::remove_file(directory.join("read-index"));
        if !result.success {
            let mut message = String::new();
            File::open(directory.join("stderr"))?
                .take(4096)
                .read_to_string(&mut message)?;
            return Err(error(if message.is_empty() {
                "Git command failed".to_owned()
            } else {
                message
            }));
        }
        Ok(result.path)
    }

    pub fn git(&self, args: &[&str]) -> io::Result<Vec<u8>> {
        let path = self.command(&args.iter().map(OsString::from).collect::<Vec<_>>(), None)?;
        let file = File::open(path)?;
        if file.metadata()?.len() > 2_000_000 {
            return Err(error("Git response too large; narrow the selection"));
        }
        let mut data = Vec::new();
        file.take(2_000_001).read_to_end(&mut data)?;
        self.clear_output();
        Ok(data)
    }

    pub fn text(&self, args: &[&str]) -> io::Result<String> {
        Ok(String::from_utf8_lossy(&self.git(args)?).trim().to_owned())
    }

    pub fn clear_output(&self) {
        for name in ["stdout", "stderr", "read-index"] {
            let _ = fs::remove_file(self.scratch.join(name));
        }
    }

    pub fn remove_generated(&self, path: &Path) -> io::Result<()> {
        crate::proc::reject_link(&self.root.join(".dark"))?;
        crate::proc::reject_link(path)?;
        if !path.exists() {
            return Ok(());
        }
        let base = self.root.join(".dark").canonicalize()?;
        let resolved = path.canonicalize()?;
        if resolved == base || !resolved.starts_with(&base) {
            return Err(error(
                "Refusing to remove output outside the generated directory",
            ));
        }
        if resolved.is_dir() {
            fs::remove_dir_all(resolved)
        } else {
            fs::remove_file(resolved)
        }
    }
    pub fn branch(&self) -> String {
        self.text(&["symbolic-ref", "--short", "HEAD"])
            .unwrap_or_else(|_| "detached HEAD".into())
    }
    pub fn ahead_behind(&self) -> (u64, u64) {
        let value = self
            .text(&["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
            .unwrap_or_default();
        let mut numbers = value.split_whitespace().filter_map(|n| n.parse().ok());
        (numbers.next().unwrap_or(0), numbers.next().unwrap_or(0))
    }

    pub fn safe_path(&self, path: &Path) -> io::Result<PathBuf> {
        if path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
            || path.starts_with(".git")
            || path.starts_with(".dark")
        {
            return Err(error("Unsafe repository path"));
        }
        let mut full = self.root.clone();
        for part in path.components() {
            full.push(part);
            if let Ok(meta) = fs::symlink_metadata(&full) {
                if meta.file_type().is_symlink() || (!meta.is_file() && !meta.is_dir()) {
                    return Err(error("Symlinks and special files are read-only"));
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(error("Reparse points are read-only"));
                    }
                }
            }
        }
        Ok(full)
    }

    pub fn status(&self) -> io::Result<Vec<StatusFile>> {
        let output = self.command(
            &["status", "--porcelain=v2", "-z", "--untracked-files=all"].map(OsString::from),
            None,
        )?;
        let mut input = BufReader::new(File::open(output)?);
        let mut record = Vec::new();
        let mut files = Vec::new();
        loop {
            record.clear();
            if input.read_until(0, &mut record)? == 0 {
                break;
            }
            record.pop();
            let kind = record.first().copied().unwrap_or_default();
            let fields = match kind {
                b'1' => 8,
                b'2' => 9,
                b'u' => 10,
                b'?' => 1,
                _ => continue,
            };
            let mut parts = record.splitn(fields + 1, |b| *b == b' ');
            parts.next();
            let xy = if kind == b'?' {
                b"??".as_slice()
            } else {
                parts.next().unwrap_or_default()
            };
            let path_bytes = if kind == b'?' {
                parts.next().unwrap_or_default()
            } else {
                parts.last().unwrap_or_default()
            };
            let path = path_from_bytes(path_bytes)?;
            let mut display_path = path.to_string_lossy().into_owned();
            let mut renamed_from = None;
            if kind == b'2' {
                let mut original = Vec::new();
                input.read_until(0, &mut original)?;
                original.pop();
                let original = path_from_bytes(&original)?;
                display_path = format!("{} -> {}", original.display(), path.display());
                renamed_from = Some(original);
            }
            if path.starts_with(".dark") {
                continue;
            }
            let staged = kind != b'?' && xy.first().is_some_and(|b| *b != b'.');
            let changed = kind != b'?' && xy.get(1).is_some_and(|b| *b != b'.');
            files.push(StatusFile {
                display_path,
                path,
                status: if kind == b'2' {
                    'A'
                } else if kind == b'?' {
                    '?'
                } else {
                    xy.iter().copied().find(|b| *b != b'.').unwrap_or(b'M') as char
                },
                staged,
                changed,
                untracked: kind == b'?',
                conflict: kind == b'u',
                added: None,
                deleted: None,
            });
            // Review uses --no-renames, so expose both halves to avoid hiding the deleted source.
            if let Some(original) = renamed_from.filter(|p| !p.starts_with(".dark")) {
                files.push(StatusFile {
                    display_path: original.to_string_lossy().into_owned(),
                    path: original,
                    status: 'D',
                    staged,
                    changed,
                    untracked: false,
                    conflict: false,
                    added: None,
                    deleted: None,
                });
            }
        }
        for cached in [false, true] {
            let mut args = vec![
                "diff",
                "--numstat",
                "-z",
                "--no-renames",
                "--no-ext-diff",
                "--no-textconv",
            ];
            if cached {
                args.push("--cached");
            }
            let stats = self.git(&args)?;
            for record in stats.split(|b| *b == 0).filter(|s| !s.is_empty()) {
                let mut p = record.splitn(3, |b| *b == b'\t');
                let add = p.next().unwrap_or_default();
                let del = p.next().unwrap_or_default();
                let path = path_from_bytes(p.next().unwrap_or_default())?;
                if let Some(file) = files.iter_mut().find(|f| f.path == path) {
                    file.added = std::str::from_utf8(add).ok().and_then(|n| n.parse().ok());
                    file.deleted = std::str::from_utf8(del).ok().and_then(|n| n.parse().ok());
                }
            }
        }
        Ok(files)
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = self.remove_generated(&self.scratch);
    }
}
