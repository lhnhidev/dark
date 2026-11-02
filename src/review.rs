//! Review baseline and raw-byte recovery snapshots never use the user's index.
use crate::git::{error, path_from_bytes, Repo, StatusFile};
use std::cell::Cell;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Add,
    Delete,
}
#[derive(Clone, Debug)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_line: Option<u64>,
    pub new_line: Option<u64>,
    pub text: String,
}
#[derive(Clone, Debug)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
    pub old_start: u64,
    pub old_count: u64,
    pub new_start: u64,
    pub new_count: u64,
    pub bytes: Range<usize>,
}
#[derive(Clone, Debug, Default)]
pub struct FileDiff {
    pub patch: Vec<u8>,
    pub hunks: Vec<Hunk>,
    pub binary: bool,
}
#[derive(Clone, Debug)]
pub struct SnapshotOptions {
    pub retain_sessions: usize,
    pub skip_large_untracked: bool,
}
impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            retain_sessions: std::env::var("DARK_SNAPSHOT_KEEP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10)
                .max(1),
            skip_large_untracked: false,
        }
    }
}
#[derive(Clone, Debug)]
pub struct OversizedFile {
    pub path: PathBuf,
    pub bytes: u64,
}
pub struct Review {
    pub repo: Repo,
    pub files: Vec<StatusFile>,
    pub baseline: String,
    pub startup: String,
    pub needs_confirmation: bool,
    pub oversized: Vec<OversizedFile>,
    pub untracked_count: usize,
    session: String,
    skipped: Vec<PathBuf>,
    turn: Cell<u64>,
}

fn session_id() -> String {
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{:020}-{}-{:020}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        std::process::id(),
        SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

impl Review {
    pub fn open(repo: Repo, options: SnapshotOptions) -> io::Result<Self> {
        let files = repo.status()?;
        let session = session_id();
        let untracked_count = files.iter().filter(|f| f.untracked).count();
        let mut oversized = Vec::new();
        let mut skipped = Vec::new();
        for file in files.iter().filter(|f| f.untracked) {
            let bytes = fs::symlink_metadata(repo.root.join(&file.path))?.len();
            if bytes > 50 * 1024 * 1024 || untracked_count > 1000 {
                oversized.push(OversizedFile {
                    path: file.path.clone(),
                    bytes,
                });
                skipped.push(file.path.clone());
            }
        }
        let needs_confirmation = !oversized.is_empty() && !options.skip_large_untracked;
        let baseline = repo.text(&["rev-parse", "HEAD^{tree}"]).unwrap_or_default();
        let mut review = Self {
            repo,
            files,
            baseline,
            startup: String::new(),
            needs_confirmation,
            oversized,
            untracked_count,
            session,
            skipped,
            turn: Cell::new(0),
        };
        if needs_confirmation {
            return Ok(review);
        }
        review.finish_snapshot(options.retain_sessions)?;
        Ok(review)
    }

    pub fn skip_oversized(&mut self, retain_sessions: usize) -> io::Result<()> {
        self.finish_snapshot(retain_sessions)?;
        self.needs_confirmation = false;
        Ok(())
    }

    fn finish_snapshot(&mut self, retention: usize) -> io::Result<()> {
        let active = self.active_path();
        crate::proc::reject_link(&active)?;
        fs::write(&active, std::process::id().to_string())?;
        self.startup = match self.snapshot("startup") {
            Ok(tree) => tree,
            Err(failure) => {
                let _ = self
                    .repo
                    .remove_generated(&self.permission_path(Path::new("")));
                let _ = self.repo.remove_generated(&active);
                self.session = session_id();
                return Err(failure);
            }
        };
        if self.baseline.is_empty() {
            self.baseline = self.repo.text(&["mktree"])?;
        }
        self.retain(retention.max(1))
    }

    fn retain(&self, keep: usize) -> io::Result<()> {
        let refs = self.repo.text(&[
            "for-each-ref",
            "--format=%(refname)",
            "refs/dark/snapshots/",
        ])?;
        let mut sessions: Vec<&str> = refs
            .lines()
            .filter_map(|r| {
                r.strip_prefix("refs/dark/snapshots/")
                    .and_then(|r| r.split('/').next())
            })
            .collect();
        sessions.sort_unstable();
        sessions.dedup();
        let expired = sessions.len().saturating_sub(keep);
        let expired: Vec<&str> = sessions[..expired]
            .iter()
            .copied()
            .filter(|session| {
                let active = self.repo.root.join(format!(".dark/active-{session}"));
                let live = fs::read_to_string(active)
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .is_some_and(crate::proc::process_alive);
                !live
            })
            .collect();
        for reference in refs.lines().filter(|r| {
            expired
                .iter()
                .any(|s| r.starts_with(&format!("refs/dark/snapshots/{s}/")))
        }) {
            self.repo.git(&["update-ref", "-d", reference])?;
        }
        for session in expired {
            for name in [
                format!("permissions-{session}"),
                format!("active-{session}"),
                format!("snapshot-index-{session}"),
                format!("snapshot-paths-{session}"),
                format!("undo-token-{session}"),
                format!("reject-{session}.patch"),
            ] {
                self.repo
                    .remove_generated(&self.repo.root.join(".dark").join(name))?;
            }
            for entry in fs::read_dir(self.repo.root.join(".dark"))? {
                let entry = entry?;
                if entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&format!("undo-{session}-"))
                {
                    self.repo.remove_generated(&entry.path())?;
                }
            }
        }
        // A crashed process can leave disposable outputs without a completed ref.
        // Preserve recovery journals for retained sessions, and never clean a live process.
        for entry in fs::read_dir(self.repo.root.join(".dark"))? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(pid) = name
                .strip_prefix("process-")
                .and_then(|tail| tail.split('-').next())
                .and_then(|pid| pid.parse().ok())
            {
                if !crate::proc::process_alive(pid) {
                    self.repo.remove_generated(&entry.path())?;
                }
            }
            if let Some(session) = name
                .strip_prefix("active-")
                .or_else(|| name.strip_prefix("permissions-"))
            {
                let active = self.repo.root.join(format!(".dark/active-{session}"));
                let live = fs::read_to_string(&active)
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .is_some_and(crate::proc::process_alive);
                if live {
                    continue;
                }
                for name in [
                    format!("active-{session}"),
                    format!("snapshot-index-{session}"),
                    format!("snapshot-index-{session}.lock"),
                    format!("snapshot-paths-{session}"),
                    format!("undo-token-{session}"),
                    format!("reject-{session}.patch"),
                ] {
                    self.repo
                        .remove_generated(&self.repo.root.join(".dark").join(name))?;
                }
                if !sessions.contains(&session) {
                    self.repo.remove_generated(
                        &self.repo.root.join(format!(".dark/permissions-{session}")),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn snapshot(&self, turn: &str) -> io::Result<String> {
        if turn != "startup" {
            let files = self.repo.status()?;
            let count = files
                .iter()
                .filter(|f| f.untracked && !self.skipped.contains(&f.path))
                .count();
            if count > 1000
                || files
                    .iter()
                    .filter(|f| f.untracked && !self.skipped.contains(&f.path))
                    .any(|f| {
                        fs::metadata(self.repo.root.join(&f.path))
                            .is_ok_and(|m| m.len() > 50 * 1024 * 1024)
                    })
            {
                return Err(error("New untracked files exceed the recovery limit (50 MiB per file or 1000 files). Reopen dark and choose whether to skip them before changing files"));
            }
        }
        let index = self
            .repo
            .root
            .join(format!(".dark/snapshot-index-{}", self.session));
        let mut cleanup = RestoreTemps(vec![index.clone(), index.with_extension("lock")]);
        let _ = fs::remove_file(&index);
        self.repo
            .command(&["read-tree", "--empty"].map(OsString::from), Some(&index))?;
        let output = self.repo.command(
            &[
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
            ]
            .map(OsString::from),
            None,
        )?;
        // Commands reuse their output file, so keep this listing on disk while hashing.
        let listing = self
            .repo
            .root
            .join(format!(".dark/snapshot-paths-{}", self.session));
        cleanup.0.push(listing.clone());
        crate::proc::reject_link(&listing)?;
        fs::copy(output, &listing)?;
        let mut reader = BufReader::new(File::open(listing)?);
        let mut bytes = Vec::new();
        loop {
            bytes.clear();
            if reader.read_until(0, &mut bytes)? == 0 {
                break;
            }
            bytes.pop();
            let path = path_from_bytes(&bytes)?;
            if path.starts_with(".dark") || self.skipped.contains(&path) {
                continue;
            }
            let full = self.repo.safe_path(&path)?;
            let metadata = match fs::metadata(&full) {
                Ok(m) => m,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            if !metadata.is_file() {
                return Err(error(
                    "Submodules and special files are not supported for recovery snapshots",
                ));
            }
            let mut args = vec![
                OsString::from("hash-object"),
                OsString::from("-w"),
                OsString::from("--no-filters"),
                OsString::from("--"),
            ];
            args.push(full.into_os_string());
            let hash = fs::read(self.repo.command(&args, None)?)?;
            let hash = String::from_utf8_lossy(&hash).trim().to_owned();
            let mut verify_args = args.clone();
            verify_args.remove(1);
            let verified = fs::read(self.repo.command(&verify_args, None)?)?;
            if String::from_utf8_lossy(&verified).trim() != hash {
                return Err(error("A file changed while the snapshot was created; retry after the writer finishes"));
            }
            let after = fs::metadata(self.repo.root.join(&path))?;
            if metadata.len() != after.len() || metadata.modified()? != after.modified()? {
                return Err(error("A file changed while the snapshot was created; retry after the writer finishes"));
            }
            let mode = file_mode(&metadata);
            if turn == "startup" {
                let permissions = self.permission_path(&path);
                if let Some(parent) = permissions.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(permissions)?;
                file.write_all(&permission_value(&metadata).to_le_bytes())?;
            }
            self.repo.command(
                &[
                    OsString::from("update-index"),
                    OsString::from("--add"),
                    OsString::from("--cacheinfo"),
                    OsString::from(mode),
                    OsString::from(hash),
                    path.into_os_string(),
                ],
                Some(&index),
            )?;
        }
        let output = self
            .repo
            .command(&[OsString::from("write-tree")], Some(&index))?;
        let tree = String::from_utf8_lossy(&fs::read(output)?)
            .trim()
            .to_owned();
        let number = self.turn.get();
        self.turn.set(number + 1);
        self.repo.git(&[
            "update-ref",
            &format!("refs/dark/snapshots/{}/{number:06}-{turn}", self.session),
            &tree,
        ])?;
        let _ = fs::remove_file(index);
        Ok(tree)
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        let files = self.repo.status()?;
        let baseline = match self.repo.text(&["rev-parse", "--verify", "HEAD^{tree}"]) {
            Ok(tree) => tree,
            Err(head_error) => {
                let status = self.repo.text(&[
                    "status",
                    "--porcelain=v2",
                    "--branch",
                    "--untracked-files=no",
                ])?;
                if status.lines().any(|line| line == "# branch.oid (initial)") {
                    self.repo.text(&["mktree"])?
                } else {
                    return Err(head_error);
                }
            }
        };
        self.files = files;
        self.baseline = baseline;
        Ok(())
    }

    pub fn diff(&self, path: &Path) -> io::Result<FileDiff> {
        let full = self.repo.safe_path(path)?;
        let args = vec![
            OsString::from("diff"),
            OsString::from("--no-ext-diff"),
            OsString::from("--no-textconv"),
            OsString::from("--no-renames"),
            OsString::from("--binary"),
            OsString::from(&self.baseline),
            OsString::from("--"),
            path.as_os_str().to_owned(),
        ];
        let output = self.repo.command(&args, None)?;
        if fs::metadata(&output)?.len() > 512 * 1024 {
            return Err(error(
                "Diff is larger than 512 KiB; open this file in the external editor",
            ));
        }
        let mut patch = fs::read(output)?;
        // git diff <tree> omits untracked files. Build a raw, Git-compatible text patch lazily.
        if patch.is_empty() && self.files.iter().any(|f| f.path == path && f.untracked) {
            let file = File::open(full)?;
            if file.metadata()?.len() > 256 * 1024 {
                return Err(error(
                    "Untracked file is larger than 256 KiB; use the external editor",
                ));
            }
            let mut contents = Vec::new();
            file.take(256 * 1024 + 1).read_to_end(&mut contents)?;
            if contents.contains(&0) {
                self.repo.clear_output();
                return Ok(FileDiff {
                    binary: true,
                    ..FileDiff::default()
                });
            }
            let display = quote_path(path)?;
            patch.extend_from_slice(format!("diff --git a/{display} b/{display}\nnew file mode 100644\n--- /dev/null\n+++ b/{display}\n@@ -0,0 +1,{} @@\n",contents.split_inclusive(|b| *b==b'\n').count()).as_bytes());
            for line in contents.split_inclusive(|b| *b == b'\n') {
                patch.push(b'+');
                patch.extend_from_slice(line);
            }
            if !contents.is_empty() && !contents.ends_with(b"\n") {
                patch.extend_from_slice(b"\n\\ No newline at end of file\n");
            }
        }
        self.repo.clear_output();
        if std::str::from_utf8(&patch).is_err() {
            return Ok(FileDiff {
                binary: true,
                ..FileDiff::default()
            });
        }
        Ok(parse_diff(patch))
    }

    pub fn reject(&mut self, path: &Path, shown: &FileDiff, hunk_index: usize) -> io::Result<()> {
        self.ensure_ready()?;
        if self.skipped.iter().any(|skipped| skipped == path) {
            return Err(error(
                "This file was skipped from recovery snapshots and is read-only for this session",
            ));
        }
        self.repo.safe_path(path)?;
        self.refresh()?;
        if self.files.iter().any(|f| f.conflict) {
            return Err(error("Resolve Git conflicts before changing files"));
        }
        let current = self.diff(path)?;
        if current.patch != shown.patch {
            return Err(error(
                "The file changed since this diff was shown; refresh and review it again",
            ));
        }
        let hunk = shown
            .hunks
            .get(hunk_index)
            .ok_or_else(|| error("No hunk selected"))?;
        if hunk.new_count
            != hunk
                .lines
                .iter()
                .filter(|line| line.kind != LineKind::Delete)
                .count() as u64
            || hunk.old_count
                != hunk
                    .lines
                    .iter()
                    .filter(|line| line.kind != LineKind::Add)
                    .count() as u64
        {
            return Err(error(
                "The patch is incomplete; refresh the diff before rejecting it",
            ));
        }
        if self.files.iter().any(|f| f.path == path && f.staged) {
            if self
                .repo
                .text(&["rev-parse", "HEAD^{tree}"])
                .unwrap_or_default()
                != self.baseline
            {
                return Err(error("HEAD changed after opening dark; reopen the repository before rejecting staged content"));
            }
            let args = vec![
                OsString::from("diff"),
                OsString::from("--cached"),
                OsString::from("--no-ext-diff"),
                OsString::from("--no-textconv"),
                OsString::from("--no-renames"),
                OsString::from("-U0"),
                OsString::from("--"),
                path.as_os_str().to_owned(),
            ];
            let staged = parse_diff(fs::read(self.repo.command(&args, None)?)?);
            if staged.hunks.is_empty() || staged.hunks.iter().any(|s| overlaps(hunk, s)) {
                return Err(error(
                    "This hunk overlaps staged changes; unstage those changes before rejecting it",
                ));
            }
        }
        let first = shown
            .hunks
            .first()
            .ok_or_else(|| error("Binary files cannot be rejected by hunk"))?
            .bytes
            .start;
        let patch_path = self
            .repo
            .root
            .join(format!(".dark/reject-{}.patch", self.session));
        let _cleanup = RestoreTemps(vec![patch_path.clone()]);
        crate::proc::reject_link(&patch_path)?;
        let mut patch = File::create(&patch_path)?;
        patch.write_all(&shown.patch[..first])?;
        patch.write_all(&shown.patch[hunk.bytes.clone()])?;
        patch.sync_all()?;
        let args = vec![
            OsString::from("apply"),
            OsString::from("--reverse"),
            OsString::from("--check"),
            OsString::from("--no-index"),
            OsString::from("--whitespace=nowarn"),
            patch_path.as_os_str().to_owned(),
        ];
        self.repo.command(&args, None)?;
        self.snapshot("before-reject")?;
        let mut args = args;
        args.remove(2);
        self.repo.command(&args, None)?;
        self.refresh()
    }

    fn ensure_ready(&self) -> io::Result<()> {
        if self.needs_confirmation || self.startup.is_empty() {
            return Err(error(
                "Confirm the oversized untracked-file warning before changing files",
            ));
        }
        Ok(())
    }

    pub fn undo_plan(&self) -> io::Result<Vec<PathBuf>> {
        self.ensure_ready()?;
        let entries = self.tree_entries()?;
        let mut changed = Vec::new();
        for (mode, hash, path) in entries {
            let full = self.repo.safe_path(&path)?;
            if !full.exists() {
                changed.push(path);
                continue;
            }
            let metadata = fs::metadata(&full)?;
            let saved = fs::read(self.permission_path(&path))?;
            let mode_changed =
                file_mode(&metadata) != mode || saved != permission_value(&metadata).to_le_bytes();
            let args = vec![
                OsString::from("hash-object"),
                OsString::from("--no-filters"),
                OsString::from("--"),
                full.into_os_string(),
            ];
            let current = String::from_utf8_lossy(&fs::read(self.repo.command(&args, None)?)?)
                .trim()
                .to_owned();
            if current != hash || mode_changed {
                changed.push(path);
            }
        }
        Ok(changed)
    }

    pub fn undo_token(&self) -> io::Result<String> {
        self.ensure_ready()?;
        let manifest = self
            .repo
            .root
            .join(format!(".dark/undo-token-{}", self.session));
        let _cleanup = RestoreTemps(vec![manifest.clone()]);
        crate::proc::reject_link(&manifest)?;
        let mut file = File::create(&manifest)?;
        for (_, _, path) in self.tree_entries()? {
            let full = self.repo.safe_path(&path)?;
            writeln!(file, "{}", path.display())?;
            if !full.exists() {
                writeln!(file, "missing")?;
                continue;
            }
            let metadata = fs::metadata(&full)?;
            let hash = fs::read(self.repo.command(
                &[
                    OsString::from("hash-object"),
                    OsString::from("--no-filters"),
                    OsString::from("--"),
                    full.into_os_string(),
                ],
                None,
            )?)?;
            file.write_all(&hash)?;
            writeln!(file, "{}", permission_value(&metadata))?;
        }
        file.sync_all()?;
        let hash = fs::read(self.repo.command(
            &[
                OsString::from("hash-object"),
                OsString::from("--no-filters"),
                OsString::from("--"),
                manifest.into_os_string(),
            ],
            None,
        )?)?;
        Ok(String::from_utf8_lossy(&hash).trim().to_owned())
    }

    pub fn undo_checked(&mut self, expected: &str) -> io::Result<()> {
        self.ensure_ready()?;
        if self.undo_token()? != expected {
            return Err(error(
                "Files changed after the restore confirmation was opened; review the list again",
            ));
        }
        self.refresh()?;
        if self.files.iter().any(|f| f.conflict) {
            return Err(error("Resolve conflicts before restoring a snapshot"));
        }
        let plan = self.undo_plan()?;
        let entries = self.tree_entries()?;
        self.snapshot("before-undo")?;
        // Prepare every replacement before modifying any working file; the recovery ref survives failures.
        let journal = self.repo.root.join(format!(
            ".dark/undo-{}-{}",
            self.session,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir(&journal)?;
        let mut prepared = Vec::new();
        let mut cleanup = RestoreTemps(Vec::new());
        for (number, (mode, hash, path)) in entries
            .iter()
            .filter(|(_, _, p)| plan.contains(p))
            .enumerate()
        {
            let full = self.repo.safe_path(path)?;
            let existed = full.exists();
            if existed {
                fs::copy(&full, journal.join(format!("backup-{number}")))?;
            }
            let output = self.repo.command(
                &[
                    OsString::from("cat-file"),
                    OsString::from("blob"),
                    OsString::from(hash),
                ],
                None,
            )?;
            fs::copy(output, journal.join(number.to_string()))?;
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent)?;
            }
            let temporary = full.with_file_name(format!(".dark-restore-{}-{number}", self.session));
            let mut target = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            cleanup.0.push(temporary.clone());
            io::copy(
                &mut File::open(journal.join(number.to_string()))?,
                &mut target,
            )?;
            target.sync_all()?;
            set_mode(&temporary, mode)?;
            restore_permissions(&temporary, &fs::read(self.permission_path(path))?)?;
            prepared.push((full, temporary, existed));
        }
        let mut log = File::create(journal.join("journal"))?;
        writeln!(
            log,
            "Recovery ref: refs/dark/snapshots/{}/{:06}-before-undo",
            self.session,
            self.turn.get() - 1
        )?;
        for path in &plan {
            writeln!(log, "{}", path.display())?;
        }
        log.sync_all()?;
        if self.undo_token()? != expected {
            return Err(error(
                "Files changed while restore was being prepared; review the list again",
            ));
        }
        for (number, (full, _, existed)) in prepared.iter().enumerate() {
            if *existed {
                if !files_equal(full, &journal.join(format!("backup-{number}")))? {
                    return Err(error(
                        "A file changed while preparing restore; retry after the writer finishes",
                    ));
                }
            } else if full.exists() {
                return Err(error(
                    "A file appeared while preparing restore; refresh first",
                ));
            }
        }
        for (number, (full, temporary, _)) in prepared.iter().enumerate() {
            let result = fs::rename(temporary, full);
            if let Err(failure) = result {
                let mut rollback_error = None;
                for (previous, (target, _, was_present)) in
                    prepared.iter().take(number).enumerate().rev()
                {
                    let restored = if *was_present {
                        fs::rename(journal.join(format!("backup-{previous}")), target)
                    } else if target.exists() {
                        fs::remove_file(target)
                    } else {
                        Ok(())
                    };
                    if let Err(e) = restored {
                        rollback_error = Some(e);
                    }
                }
                return Err(error(format!(
                    "Restore failed: {failure}. {} Journal: {}",
                    rollback_error
                        .map(|e| format!("Automatic recovery failed: {e}."))
                        .unwrap_or_else(|| "Original files were recovered.".into()),
                    journal.display()
                )));
            }
        }
        self.repo.remove_generated(&journal)?;
        self.refresh()
    }

    fn tree_entries(&self) -> io::Result<Vec<(String, String, PathBuf)>> {
        let listing = self.repo.git(&["ls-tree", "-rz", &self.startup])?;
        let mut entries = Vec::new();
        for entry in listing.split(|b| *b == 0).filter(|e| !e.is_empty()) {
            let (meta, path) = entry
                .split_once_byte(b'\t')
                .ok_or_else(|| error("Invalid snapshot entry"))?;
            let mut fields = meta.split(|b| *b == b' ');
            let mode = String::from_utf8_lossy(fields.next().unwrap_or_default()).into_owned();
            fields.next();
            let hash = String::from_utf8_lossy(fields.next().unwrap_or_default()).into_owned();
            if mode != "100644" && mode != "100755" {
                return Err(error("Special snapshot entries cannot be restored safely"));
            }
            entries.push((mode, hash, path_from_bytes(path)?));
        }
        Ok(entries)
    }

    fn permission_path(&self, path: &Path) -> PathBuf {
        self.repo
            .root
            .join(format!(".dark/permissions-{}", self.session))
            .join(path)
    }
    fn active_path(&self) -> PathBuf {
        self.repo
            .root
            .join(format!(".dark/active-{}", self.session))
    }
}

impl Drop for Review {
    fn drop(&mut self) {
        let _ = self.repo.remove_generated(&self.active_path());
        self.repo.clear_output();
    }
}

struct RestoreTemps(Vec<PathBuf>);
impl Drop for RestoreTemps {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}

fn files_equal(a: &Path, b: &Path) -> io::Result<bool> {
    let mut a = File::open(a)?;
    let mut b = File::open(b)?;
    if a.metadata()?.len() != b.metadata()?.len() {
        return Ok(false);
    }
    let mut left = [0u8; 8192];
    let mut right = [0u8; 8192];
    loop {
        let count = a.read(&mut left)?;
        b.read_exact(&mut right[..count])?;
        if left[..count] != right[..count] {
            return Ok(false);
        }
        if count == 0 {
            return Ok(true);
        }
    }
}

trait SplitByte {
    fn split_once_byte(&self, byte: u8) -> Option<(&[u8], &[u8])>;
}
impl SplitByte for [u8] {
    fn split_once_byte(&self, byte: u8) -> Option<(&[u8], &[u8])> {
        self.iter()
            .position(|b| *b == byte)
            .map(|i| (&self[..i], &self[i + 1..]))
    }
}
fn quote_path(path: &Path) -> io::Result<String> {
    let name = path
        .to_str()
        .ok_or_else(|| error("Untracked non-Unicode names are read-only"))?;
    if name.contains(['\n', '\r', '\t', '"', '\\']) {
        return Err(error("This untracked filename requires external review"));
    }
    Ok(name.to_owned())
}
fn file_mode(metadata: &fs::Metadata) -> &'static str {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 != 0 {
            return "100755";
        }
    }
    #[cfg(not(unix))]
    let _ = metadata;
    "100644"
}
fn set_mode(path: &Path, mode: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if mode == "100755" { 0o755 } else { 0o644 }),
        )?;
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
}
fn permission_value(metadata: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        u32::from(metadata.permissions().readonly())
    }
}
fn restore_permissions(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let value = u32::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| error("Recovery permission metadata is invalid"))?,
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(value))?;
    }
    #[cfg(not(unix))]
    {
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_readonly(value != 0);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}
fn overlaps(a: &Hunk, b: &Hunk) -> bool {
    a.old_start.saturating_sub(1) <= b.old_start + b.old_count
        && b.old_start.saturating_sub(1) <= a.old_start + a.old_count
}
pub fn parse_diff(patch: Vec<u8>) -> FileDiff {
    let binary = patch.windows(12).any(|w| w == b"GIT binary p")
        || patch.windows(12).any(|w| w == b"Binary files");
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut offset = 0;
    let mut old = 0;
    let mut new = 0;
    for bytes in patch.split_inclusive(|b| *b == b'\n') {
        let line = String::from_utf8_lossy(bytes);
        let text = line.trim_end_matches(['\n', '\r']);
        if text.starts_with("@@ ") {
            if let Some(previous) = hunks.last_mut() {
                previous.bytes.end = offset;
            }
            let mut fields = text.split_whitespace();
            fields.next();
            let (old_start, old_count) = parse_span(fields.next().unwrap_or("-0"));
            let (new_start, new_count) = parse_span(fields.next().unwrap_or("+0"));
            old = old_start;
            new = new_start;
            hunks.push(Hunk {
                header: text.to_owned(),
                lines: Vec::new(),
                old_start,
                old_count,
                new_start,
                new_count,
                bytes: offset..patch.len(),
            });
        } else if let Some(hunk) = hunks.last_mut() {
            let (kind, old_line, new_line) = match bytes.first() {
                Some(b'+') => {
                    let n = new;
                    new += 1;
                    (LineKind::Add, None, Some(n))
                }
                Some(b'-') => {
                    let n = old;
                    old += 1;
                    (LineKind::Delete, Some(n), None)
                }
                Some(b' ') => {
                    let o = old;
                    let n = new;
                    old += 1;
                    new += 1;
                    (LineKind::Context, Some(o), Some(n))
                }
                _ => {
                    offset += bytes.len();
                    continue;
                }
            };
            hunk.lines.push(DiffLine {
                kind,
                old_line,
                new_line,
                text: text.get(1..).unwrap_or_default().to_owned(),
            });
        }
        offset += bytes.len();
    }
    FileDiff {
        patch,
        hunks,
        binary,
    }
}
fn parse_span(span: &str) -> (u64, u64) {
    let mut numbers = span.get(1..).unwrap_or_default().split(',');
    (
        numbers.next().and_then(|n| n.parse().ok()).unwrap_or(0),
        numbers.next().map(|n| n.parse().unwrap_or(0)).unwrap_or(1),
    )
}
