use super::{git_common_dir, git_dir, metadata_path, worktree_path};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_int, c_void, CString, OsStr};
use std::fs;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

unsafe extern "C" {
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
    fn inotify_rm_watch(fd: c_int, wd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn fcntl(fd: c_int, command: c_int, ...) -> c_int;
}

pub enum Watcher {
    Native(Native),
    Bridge(Bridge),
}
impl Watcher {
    pub fn new(root: &Path) -> io::Result<Self> {
        let root = fs::canonicalize(root)?;
        let drive = root.as_os_str().as_bytes();
        let wsl = std::env::var_os("WSL_INTEROP").is_some()
            || fs::read_to_string("/proc/sys/kernel/osrelease")
                .is_ok_and(|s| s.to_ascii_lowercase().contains("microsoft"));
        if wsl
            && drive.len() >= 6
            && drive.starts_with(b"/mnt/")
            && drive[5].is_ascii_alphabetic()
            && (drive.len() == 6 || drive[6] == b'/')
        {
            return Bridge::new(&root).map(Self::Bridge);
        }
        Native::new(&root).map(Self::Native)
    }
    pub fn take_changed(&mut self) -> io::Result<bool> {
        match self {
            Self::Native(w) => w.take_changed(),
            Self::Bridge(w) => w.take_changed(),
        }
    }
}

struct Entry {
    path: PathBuf,
    metadata_root: Option<PathBuf>,
}
pub struct Native {
    fd: OwnedFd,
    root: PathBuf,
    entries: BTreeMap<i32, Entry>,
    buffer: Box<[u8; 16384]>,
}
impl Native {
    fn new(root: &Path) -> io::Result<Self> {
        let fd = unsafe { inotify_init1(0x800 | 0x80000) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut this = Self {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
            root: root.to_owned(),
            entries: BTreeMap::new(),
            buffer: Box::new([0; 16384]),
        };
        this.add_tree(root, None)?;
        let git = git_dir(root)?;
        this.add_tree(&git, Some(&git))?;
        let common = git_common_dir(root)?;
        if common != git {
            this.add_tree(&common, Some(&common))?;
        }
        Ok(this)
    }
    fn ignored(&self, path: &Path) -> io::Result<bool> {
        let status = Command::new("git")
            .current_dir(&self.root)
            .args(["check-ignore", "--quiet", "--"])
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        match status.code() {
            Some(0) => {
                // An ignored directory can still contain tracked files.
                let tracked = Command::new("git")
                    .current_dir(&self.root)
                    .args(["ls-files", "--error-unmatch", "--"])
                    .arg(path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()?;
                match tracked.code() {
                    Some(0) => Ok(false),
                    Some(1) => Ok(true),
                    _ => Err(io::Error::other(
                        "Cannot inspect tracked directories for file notifications",
                    )),
                }
            }
            Some(1) => Ok(false),
            _ => Err(io::Error::other(
                "Cannot check ignored directories for file notifications",
            )),
        }
    }
    fn add_tree(&mut self, path: &Path, metadata: Option<&Path>) -> io::Result<()> {
        let info = match fs::symlink_metadata(path) {
            Ok(info) => info,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        if !info.is_dir() || info.file_type().is_symlink() {
            return Ok(());
        }
        if let Some(base) = metadata {
            let relative = path.strip_prefix(base).unwrap_or(path);
            if !relative.as_os_str().is_empty()
                && relative != Path::new("refs")
                && relative != Path::new("info")
                && !relative.starts_with("refs/heads")
            {
                return Ok(());
            }
        } else if path != self.root
            && (!worktree_path(path.strip_prefix(&self.root).unwrap_or(path))
                || self.ignored(path)?)
        {
            return Ok(());
        }
        let name = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| io::Error::other("Invalid directory path"))?;
        // Only directory entries, writes and renames; opening/reading a file cannot trigger refresh.
        let wd = unsafe {
            inotify_add_watch(
                self.fd.as_raw_fd(),
                name.as_ptr(),
                0x00000fce | 0x01000000 | 0x02000000,
            )
        };
        if wd < 0 {
            return Err(io::Error::last_os_error());
        }
        self.entries.insert(
            wd,
            Entry {
                path: path.to_owned(),
                metadata_root: metadata.map(Path::to_owned),
            },
        );
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                self.add_tree(&entry.path(), metadata)?;
            }
        }
        Ok(())
    }
    fn remove_tree(&mut self, path: &Path) {
        let removed: Vec<i32> = self
            .entries
            .iter()
            .filter(|(_, e)| e.path.starts_with(path))
            .map(|(&wd, _)| wd)
            .collect();
        for wd in removed {
            self.entries.remove(&wd);
            unsafe {
                inotify_rm_watch(self.fd.as_raw_fd(), wd);
            }
        }
    }
    fn take_changed(&mut self) -> io::Result<bool> {
        let mut changed = false;
        // Bound work per UI turn. Remaining notifications stay in the kernel queue.
        for _ in 0..8 {
            let count = unsafe {
                read(
                    self.fd.as_raw_fd(),
                    self.buffer.as_mut_ptr().cast(),
                    self.buffer.len(),
                )
            };
            if count < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    break;
                }
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
            if count == 0 {
                return Err(io::Error::other("File notification stream closed"));
            }
            let mut offset = 0;
            while offset + 16 <= count as usize {
                let b = &self.buffer[offset..count as usize];
                let wd = i32::from_ne_bytes(b[..4].try_into().unwrap());
                let mask = u32::from_ne_bytes(b[4..8].try_into().unwrap());
                let len = u32::from_ne_bytes(b[12..16].try_into().unwrap()) as usize;
                if len > b.len() - 16 {
                    return Err(io::Error::other("Invalid file notification"));
                }
                let name = &b[16..16 + len];
                let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
                let entry = self.entries.get(&wd).map(|e| {
                    (
                        e.path.join(OsStr::from_bytes(name)),
                        e.metadata_root.clone(),
                    )
                });
                offset += 16 + len;
                if mask & 0x4000 != 0 {
                    // Overflow may include newly created directories. Re-register once,
                    // driven by this explicit OS signal rather than a scan timer.
                    let root = self.root.clone();
                    *self = Self::new(&root)?;
                    return Ok(true);
                }
                if mask & 0x8000 != 0 {
                    self.entries.remove(&wd);
                    continue;
                }
                let Some((path, metadata)) = entry else {
                    continue;
                };
                if mask & 0xc00 != 0
                    && (path == self.root || metadata.as_ref().is_some_and(|base| path == *base))
                {
                    return Err(io::Error::other("The watched repository or Git directory was moved or deleted; reopen dark in the repository"));
                }
                let relevant = match &metadata {
                    Some(base) => metadata_path(path.strip_prefix(base).unwrap_or(&path)),
                    None => worktree_path(path.strip_prefix(&self.root).unwrap_or(&path)),
                };
                if !relevant {
                    continue;
                }
                changed = true;
                if path.file_name().is_some_and(|name| name == ".gitignore")
                    || metadata.as_ref().is_some_and(|base| {
                        path == base.join("info/exclude") || path == base.join("index")
                    })
                {
                    // Ignore-rule changes and force-added files can expose
                    // directories that previously had no watch.
                    // Replace the descriptor, discarding stale removed-watch events.
                    let root = self.root.clone();
                    *self = Self::new(&root)?;
                    return Ok(true);
                }
                if mask & 0x40000000 != 0 {
                    if mask & (0x100 | 0x80) != 0 {
                        self.add_tree(&path, metadata.as_deref())?;
                    }
                    if mask & (0x200 | 0x40) != 0 {
                        self.remove_tree(&path);
                    }
                }
                if mask & 0xc00 != 0 {
                    self.remove_tree(&path);
                }
            }
        }
        Ok(changed)
    }
}

pub struct Bridge {
    child: Child,
    input: Option<ChildStdin>,
    output: ChildStdout,
    line: Vec<u8>,
    ready: bool,
    pending: bool,
}
impl Bridge {
    fn new(root: &Path) -> io::Result<Self> {
        let helper = if let Some(path) = std::env::var_os("DARK_WATCH_HELPER") {
            PathBuf::from(path)
        } else {
            let exe = std::env::current_exe()?;
            let candidates = [
                exe.with_file_name("dark.exe"),
                Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/dark.exe"),
            ];
            candidates.into_iter().find(|p| p.is_file()).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "WSL drive notifications need dark.exe beside dark, or DARK_WATCH_HELPER pointing to dark.exe"))?
        };
        let path = Command::new("wslpath")
            .arg("-w")
            .arg(root)
            .stdin(Stdio::null())
            .output()?;
        if !path.status.success() {
            return Err(io::Error::other(
                "Cannot translate the repository path for Windows notifications",
            ));
        }
        let path = path.stdout.strip_suffix(b"\n").unwrap_or(&path.stdout);
        let mut child = Command::new(helper)
            .arg("--watch-bridge")
            .arg(OsStr::from_bytes(path))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let mut this = Self {
            child,
            input: Some(input),
            output,
            line: Vec::with_capacity(16),
            ready: false,
            pending: false,
        };
        let flags = unsafe { fcntl(this.output.as_raw_fd(), 3) };
        if flags < 0 || unsafe { fcntl(this.output.as_raw_fd(), 4, flags | 0x800) } < 0 {
            return Err(io::Error::last_os_error());
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while !this.ready {
            this.pending |= this.take_changed()?;
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Windows notification helper did not become ready within five seconds",
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(this)
    }
    fn take_changed(&mut self) -> io::Result<bool> {
        let mut changed = std::mem::take(&mut self.pending);
        let mut bytes = [0; 256];
        for _ in 0..8 {
            match self.output.read(&mut bytes) {
                Ok(0) => {
                    return Err(io::Error::other(
                        "Windows notification helper stopped; restart dark after checking dark.exe",
                    ))
                }
                Ok(count) => {
                    for &byte in &bytes[..count] {
                        if byte == b'\n' {
                            match self.line.as_slice() {
                                b"READY" if !self.ready => self.ready = true,
                                b"CHANGED" if self.ready => changed = true,
                                _ => {
                                    return Err(io::Error::other(
                                        "Windows notification helper protocol is incompatible",
                                    ))
                                }
                            }
                            self.line.clear();
                        } else {
                            if self.line.len() >= 16 {
                                return Err(io::Error::other(
                                    "Invalid Windows notification helper response",
                                ));
                            }
                            self.line.push(byte);
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(changed)
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.input.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
