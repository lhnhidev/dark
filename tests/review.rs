#![allow(dead_code)]
#[path = "../src/git.rs"]
mod git;
#[path = "../src/proc.rs"]
mod proc;
#[path = "../src/review.rs"]
mod review;

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
static COUNTER: AtomicU64 = AtomicU64::new(0);
struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "dark-review-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        let fixture = Self(path);
        fixture.git(&["init", "-q"]);
        fixture.git(&["config", "user.name", "Test"]);
        fixture.git(&["config", "user.email", "test@example.invalid"]);
        fixture.git(&["config", "core.autocrlf", "false"]);
        fs::write(fixture.0.join("file.txt"),b"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\nthirteen\nfourteen\nfifteen\n").unwrap();
        fixture.git(&["add", "file.txt"]);
        fixture.git(&["commit", "-qm", "initial"]);
        fixture
    }
    fn git(&self, args: &[&str]) -> Vec<u8> {
        let output = Command::new("git")
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }
    fn open(&self) -> review::Review {
        review::Review::open(
            git::Repo::discover(&self.0).unwrap(),
            review::SnapshotOptions::default(),
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn raw_snapshot_undo_preserves_index_branch_and_unrelated_files() {
    let f = Fixture::new();
    fs::write(f.0.join("file.txt"), b"startup\r\nbytes\r\n").unwrap();
    f.git(&["add", "file.txt"]);
    fs::write(f.0.join("file.txt"), b"working\r\nbytes\r\n").unwrap();
    let index = fs::read(f.0.join(".git/index")).unwrap();
    let head = f.git(&["rev-parse", "HEAD"]);
    let mut r = f.open();
    assert!(!r.startup.is_empty());
    fs::write(f.0.join("file.txt"), b"after opening\n").unwrap();
    fs::write(f.0.join("new.txt"), b"keep me").unwrap();
    assert_eq!(r.undo_plan().unwrap(), vec![Path::new("file.txt")]);
    assert!(r.undo_checked("not-confirmed").is_err());
    r.undo_checked(&r.undo_token().unwrap()).unwrap();
    assert_eq!(
        fs::read(f.0.join("file.txt")).unwrap(),
        b"working\r\nbytes\r\n"
    );
    assert!(f.0.join("new.txt").exists());
    assert_eq!(index, fs::read(f.0.join(".git/index")).unwrap());
    assert_eq!(head, f.git(&["rev-parse", "HEAD"]));
}

#[test]
fn reject_is_atomic_stale_safe_and_preserves_staged_disjoint_changes() {
    let f = Fixture::new();
    let initial = fs::read_to_string(f.0.join("file.txt")).unwrap();
    fs::write(f.0.join("file.txt"), initial.replace("one\n", "staged\n")).unwrap();
    f.git(&["add", "file.txt"]);
    fs::write(
        f.0.join("file.txt"),
        initial
            .replace("one\n", "staged\n")
            .replace("fifteen\n", "working\n"),
    )
    .unwrap();
    let index = fs::read(f.0.join(".git/index")).unwrap();
    let mut r = f.open();
    let diff = r.diff(Path::new("file.txt")).unwrap();
    assert_eq!(diff.hunks.len(), 2);
    assert!(r.reject(Path::new("file.txt"), &diff, 0).is_err());
    r.reject(Path::new("file.txt"), &diff, 1).unwrap();
    assert_eq!(
        fs::read_to_string(f.0.join("file.txt")).unwrap(),
        initial.replace("one\n", "staged\n")
    );
    assert_eq!(index, fs::read(f.0.join(".git/index")).unwrap());
    let diff = r.diff(Path::new("file.txt")).unwrap();
    fs::write(f.0.join("file.txt"), b"newer change\n").unwrap();
    assert!(r.reject(Path::new("file.txt"), &diff, 0).is_err());
    assert_eq!(fs::read(f.0.join("file.txt")).unwrap(), b"newer change\n");
}

#[test]
fn untracked_unicode_and_spaces_can_be_reviewed_and_rejected() {
    let f = Fixture::new();
    let path = Path::new("new 日本 file.txt");
    fs::write(f.0.join(path), b"hello\n").unwrap();
    let mut r = f.open();
    assert!(r.files.iter().any(|v| v.path == path && v.untracked));
    let diff = r.diff(path).unwrap();
    assert_eq!(diff.hunks.len(), 1);
    r.reject(path, &diff, 0).unwrap();
    assert!(!f.0.join(path).exists());
}

#[test]
fn oversized_untracked_requires_explicit_skip() {
    let f = Fixture::new();
    fs::File::create(f.0.join("huge.bin"))
        .unwrap()
        .set_len(50 * 1024 * 1024 + 1)
        .unwrap();
    let mut r = f.open();
    assert!(r.needs_confirmation);
    assert_eq!(r.oversized.len(), 1);
    assert!(r.startup.is_empty());
    assert!(r.undo_checked("not-confirmed").is_err());
    r.skip_oversized(10).unwrap();
    assert!(!r.needs_confirmation);
    assert!(!r.startup.is_empty());
    assert!(r.undo_plan().unwrap().is_empty());
}

#[test]
fn snapshot_retention_drops_only_old_private_refs() {
    let f = Fixture::new();
    f.git(&["update-ref", "refs/dark/snapshots/000-old/startup", "HEAD"]);
    let _r = review::Review::open(
        git::Repo::discover(&f.0).unwrap(),
        review::SnapshotOptions {
            retain_sessions: 1,
            skip_large_untracked: false,
        },
    )
    .unwrap();
    let refs = String::from_utf8(f.git(&[
        "for-each-ref",
        "--format=%(refname)",
        "refs/dark/snapshots/",
    ]))
    .unwrap();
    assert!(!refs.contains("000-old"));
    assert_eq!(refs.lines().count(), 1);
}

#[test]
fn ignored_and_dark_output_are_excluded_from_recovery() {
    let f = Fixture::new();
    fs::write(f.0.join(".gitignore"), b"ignored.txt\n").unwrap();
    fs::write(f.0.join("ignored.txt"), b"secret").unwrap();
    fs::create_dir_all(f.0.join(".dark")).unwrap();
    fs::write(f.0.join(".dark/private"), b"generated").unwrap();
    let r = f.open();
    let tree = String::from_utf8(f.git(&["ls-tree", "-r", "--name-only", &r.startup])).unwrap();
    assert!(!tree.contains("ignored.txt"));
    assert!(!tree.contains(".dark/"));
    assert!(r.files.iter().all(|v| !v.path.starts_with(".dark")));
}

#[test]
fn large_diff_reports_read_only_without_mutating_files() {
    let f = Fixture::new();
    let content = vec![b'x'; 600_000];
    fs::write(f.0.join("file.txt"), &content).unwrap();
    let r = f.open();
    assert!(r
        .diff(Path::new("file.txt"))
        .unwrap_err()
        .to_string()
        .contains("external editor"));
    assert_eq!(fs::read(f.0.join("file.txt")).unwrap(), content);
}

#[test]
fn staged_rename_exposes_both_added_and_deleted_halves() {
    let f = Fixture::new();
    f.git(&["mv", "file.txt", "renamed.txt"]);
    let r = f.open();
    assert!(r
        .files
        .iter()
        .any(|file| file.path == Path::new("file.txt") && file.status == 'D' && file.staged));
    assert!(r
        .files
        .iter()
        .any(|file| file.path == Path::new("renamed.txt")
            && file.status == 'A'
            && file.display_path.contains("->")));
    assert!(!r.diff(Path::new("file.txt")).unwrap().hunks.is_empty());
}

#[test]
fn restore_confirmation_token_rejects_new_changes() {
    let f = Fixture::new();
    let mut r = f.open();
    fs::write(f.0.join("file.txt"), b"first change").unwrap();
    let token = r.undo_token().unwrap();
    fs::write(f.0.join("file.txt"), b"second change").unwrap();
    assert!(r.undo_checked(&token).is_err());
    assert_eq!(fs::read(f.0.join("file.txt")).unwrap(), b"second change");
}

#[test]
fn recovery_limits_are_checked_again_before_reject() {
    let f = Fixture::new();
    let mut r = f.open();
    fs::write(f.0.join("file.txt"), b"changed\n").unwrap();
    r.refresh().unwrap();
    let diff = r.diff(Path::new("file.txt")).unwrap();
    fs::File::create(f.0.join("huge.bin"))
        .unwrap()
        .set_len(50 * 1024 * 1024 + 1)
        .unwrap();
    assert!(r
        .reject(Path::new("file.txt"), &diff, 0)
        .unwrap_err()
        .to_string()
        .contains("recovery limit"));
    assert_eq!(fs::read(f.0.join("file.txt")).unwrap(), b"changed\n");
}

#[test]
fn successful_operations_remove_transient_output_and_drop_removes_scratch() {
    let f = Fixture::new();
    let mut r = f.open();
    fs::write(f.0.join("file.txt"), b"changed\n").unwrap();
    let token = r.undo_token().unwrap();
    r.undo_checked(&token).unwrap();
    for entry in fs::read_dir(f.0.join(".dark")).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(
            !["undo-", "reject-", "snapshot-"]
                .iter()
                .any(|prefix| name.starts_with(prefix)),
            "leftover {name}"
        );
        if name.starts_with("process-") {
            assert_eq!(fs::read_dir(entry.path()).unwrap().count(), 0);
        }
    }
    drop(r);
    assert!(fs::read_dir(f.0.join(".dark")).unwrap().all(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("permissions-")));
}

#[test]
fn retention_cleans_expired_metadata_but_preserves_live_sessions() {
    let f = Fixture::new();
    let r1 = f.open();
    let r2 = review::Review::open(
        git::Repo::discover(&f.0).unwrap(),
        review::SnapshotOptions {
            retain_sessions: 1,
            skip_large_untracked: false,
        },
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(f.git(&[
            "for-each-ref",
            "--format=%(refname)",
            "refs/dark/snapshots/"
        ]))
        .unwrap()
        .lines()
        .count(),
        2
    );
    drop(r1);
    drop(r2);
    let _r3 = review::Review::open(
        git::Repo::discover(&f.0).unwrap(),
        review::SnapshotOptions {
            retain_sessions: 1,
            skip_large_untracked: false,
        },
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(f.git(&[
            "for-each-ref",
            "--format=%(refname)",
            "refs/dark/snapshots/"
        ]))
        .unwrap()
        .lines()
        .count(),
        1
    );
    assert_eq!(
        fs::read_dir(f.0.join(".dark"))
            .unwrap()
            .filter(|e| e
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("permissions-"))
            .count(),
        1
    );
}

#[test]
fn failed_snapshot_cleans_partial_metadata_and_can_retry() {
    let f = Fixture::new();
    fs::File::create(f.0.join("huge.bin"))
        .unwrap()
        .set_len(50 * 1024 * 1024 + 1)
        .unwrap();
    let mut r = f.open();
    fs::write(f.0.join("a-first.txt"), b"saved before failure").unwrap();
    fs::remove_file(f.0.join("file.txt")).unwrap();
    fs::create_dir(f.0.join("file.txt")).unwrap();
    assert!(r.skip_oversized(10).is_err());
    assert!(fs::read_dir(f.0.join(".dark")).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("permissions-")));
    fs::remove_dir(f.0.join("file.txt")).unwrap();
    fs::write(f.0.join("file.txt"), b"retry works\n").unwrap();
    r.skip_oversized(10).unwrap();
    assert!(!r.startup.is_empty());
}

#[test]
fn private_snapshot_refs_are_not_in_default_push_to_local_remote() {
    let f = Fixture::new();
    let remote = f.0.join(".dark/remote.git");
    fs::create_dir_all(&remote).unwrap();
    f.git(&["init", "--bare", "-q", remote.to_str().unwrap()]);
    f.git(&["remote", "add", "origin", remote.to_str().unwrap()]);
    f.git(&["config", "push.default", "current"]);
    let _r = f.open();
    assert!(f
        .git(&["branch", "--list"])
        .windows(10)
        .all(|w| w != b"refs/dark/"));
    assert!(f.git(&["tag", "--list"]).is_empty());
    f.git(&["push", "-u", "origin"]);
    assert!(f
        .git(&[
            "--git-dir",
            remote.to_str().unwrap(),
            "for-each-ref",
            "--format=%(refname)",
            "refs/dark/"
        ])
        .is_empty());
}

#[test]
fn more_than_one_thousand_untracked_files_requires_skip() {
    let f = Fixture::new();
    for n in 0..1001 {
        fs::write(f.0.join(format!("untracked-{n}.txt")), b"x").unwrap();
    }
    let r = f.open();
    assert!(r.needs_confirmation);
    assert_eq!(r.untracked_count, 1001);
    assert_eq!(r.oversized.len(), 1001);
    assert!(r.startup.is_empty());
}

#[cfg(windows)]
#[test]
fn undo_failure_rolls_back_previously_restored_files() {
    use std::os::windows::fs::OpenOptionsExt;
    let f = Fixture::new();
    fs::write(f.0.join("z.txt"), b"startup z").unwrap();
    let mut r = f.open();
    fs::write(f.0.join("file.txt"), b"modified first").unwrap();
    fs::write(f.0.join("z.txt"), b"modified last").unwrap();
    let z = f.0.join("z.txt");
    let locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&z)
        .unwrap();
    assert!(r.undo_checked(&r.undo_token().unwrap()).is_err());
    assert_eq!(fs::read(f.0.join("file.txt")).unwrap(), b"modified first");
    assert_eq!(fs::read(&z).unwrap(), b"modified last");
    drop(locked);
}

#[cfg(unix)]
#[test]
fn undo_restores_exact_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let path = f.0.join("file.txt");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o751)).unwrap();
    let mut r = f.open();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    assert_eq!(r.undo_plan().unwrap(), vec![Path::new("file.txt")]);
    r.undo_checked(&r.undo_token().unwrap()).unwrap();
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o751
    );
}

#[cfg(unix)]
#[test]
fn symlinks_never_write_outside_repository() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let outside = f.0.with_extension("outside");
    fs::write(&outside, b"keep").unwrap();
    symlink(&outside, f.0.join("link.txt")).unwrap();
    assert!(review::Review::open(
        git::Repo::discover(&f.0).unwrap(),
        review::SnapshotOptions::default()
    )
    .is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"keep");
    fs::remove_file(outside).unwrap();
}
