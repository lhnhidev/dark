#![allow(dead_code)]
#[path = "../src/proc.rs"]
mod proc;
#[path = "../src/watch.rs"]
mod watch;

use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};
use watch::Watcher;

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "dark-watch-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        assert!(Command::new("git")
            .args(["init", "-q"])
            .current_dir(&path)
            .status()
            .unwrap()
            .success());
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn changed(watcher: &mut Watcher) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if watcher.take_changed().unwrap() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("filesystem change was not reported");
}
fn drain(watcher: &mut Watcher) {
    let until = Instant::now() + Duration::from_millis(300);
    while Instant::now() < until {
        watcher.take_changed().unwrap();
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn observes_writes_atomic_saves_nested_creation_and_deletion() {
    let fixture = Fixture::new();
    let file = fixture.0.join("sample.txt");
    fs::write(&file, "before\n").unwrap();
    let mut watcher = Watcher::new(&fixture.0).unwrap();
    fs::write(&file, "after\n").unwrap();
    changed(&mut watcher);
    drain(&mut watcher);
    let temporary = fixture.0.join("sample.tmp");
    fs::write(&temporary, "atomic\n").unwrap();
    // Windows rename cannot replace an existing file through std::fs::rename.
    fs::remove_file(&file).unwrap();
    fs::rename(&temporary, &file).unwrap();
    changed(&mut watcher);
    drain(&mut watcher);
    let nested = fixture.0.join("new/nested");
    fs::create_dir_all(&nested).unwrap();
    changed(&mut watcher);
    drain(&mut watcher);
    fs::write(nested.join("new.txt"), "nested\n").unwrap();
    changed(&mut watcher);
    drain(&mut watcher);
    fs::remove_file(nested.join("new.txt")).unwrap();
    changed(&mut watcher);
}

#[test]
fn ignores_dark_snapshot_activity_but_observes_git_index() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join(".dark")).unwrap();
    let mut watcher = Watcher::new(&fixture.0).unwrap();
    fs::write(fixture.0.join(".dark/snapshot"), "generated").unwrap();
    fs::create_dir_all(fixture.0.join(".git/refs/dark/snapshots")).unwrap();
    fs::write(
        fixture.0.join(".git/refs/dark/snapshots/session"),
        "generated",
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_millis(600);
    while Instant::now() < deadline {
        assert!(
            !watcher.take_changed().unwrap(),
            "generated .dark file triggered refresh"
        );
        thread::sleep(Duration::from_millis(20));
    }
    fs::write(fixture.0.join(".git/index"), "index").unwrap();
    changed(&mut watcher);
}

#[test]
fn observes_index_in_linked_worktree() {
    let fixture = Fixture::new();
    let git = |args: &[&str]| {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&fixture.0)
            .status()
            .unwrap()
            .success());
    };
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.invalid"]);
    fs::write(fixture.0.join("sample.txt"), "before\n").unwrap();
    git(&["add", "sample.txt"]);
    git(&["commit", "-qm", "initial"]);
    let linked = fixture.0.join("linked");
    git(&[
        "worktree",
        "add",
        "-qb",
        "linked-test",
        linked.to_str().unwrap(),
    ]);
    let mut watcher = Watcher::new(&linked).unwrap();
    fs::write(linked.join("sample.txt"), "after\n").unwrap();
    changed(&mut watcher);
    drain(&mut watcher);
    assert!(Command::new("git")
        .args(["add", "sample.txt"])
        .current_dir(&linked)
        .status()
        .unwrap()
        .success());
    changed(&mut watcher);
}

#[test]
fn observes_same_tree_branch_switch_and_detached_head() {
    let fixture = Fixture::new();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(&fixture.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.invalid"]);
    fs::write(fixture.0.join("sample.txt"), "initial\n").unwrap();
    git(&["add", "sample.txt"]);
    git(&["commit", "-qm", "initial"]);
    git(&["branch", "same-tree-test"]);
    let mut watcher = Watcher::new(&fixture.0).unwrap();
    git(&["checkout", "-q", "same-tree-test"]);
    changed(&mut watcher);
    drain(&mut watcher);
    git(&["checkout", "-q", "--detach"]);
    changed(&mut watcher);
}

#[test]
fn observes_nested_dark_directory() {
    let fixture = Fixture::new();
    let nested = fixture.0.join("src/.dark");
    fs::create_dir_all(&nested).unwrap();
    let file = nested.join("config");
    fs::write(&file, "before").unwrap();
    assert!(Command::new("git")
        .args(["add", "-f", "src/.dark/config"])
        .current_dir(&fixture.0)
        .status()
        .unwrap()
        .success());
    let mut watcher = Watcher::new(&fixture.0).unwrap();
    fs::write(file, "after").unwrap();
    changed(&mut watcher);
}

#[cfg(target_os = "linux")]
#[test]
fn observes_force_added_tracked_file_in_ignored_directory() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join(".gitignore"), "ignored/\n").unwrap();
    fs::create_dir_all(fixture.0.join("ignored")).unwrap();
    let file = fixture.0.join("ignored/tracked.txt");
    fs::write(&file, "before").unwrap();
    assert!(Command::new("git")
        .args(["add", "-f", "ignored/tracked.txt"])
        .current_dir(&fixture.0)
        .status()
        .unwrap()
        .success());
    let mut watcher = Watcher::new(&fixture.0).unwrap();
    fs::write(file, "after").unwrap();
    changed(&mut watcher);
}

#[cfg(target_os = "linux")]
#[test]
fn observes_files_after_ignore_rules_are_removed() {
    for rules in [".gitignore", ".git/info/exclude"] {
        let fixture = Fixture::new();
        let rule_file = fixture.0.join(rules);
        fs::write(&rule_file, "ignored/\n").unwrap();
        fs::create_dir_all(fixture.0.join("ignored")).unwrap();
        let file = fixture.0.join("ignored/file.txt");
        fs::write(&file, "before").unwrap();
        let mut watcher = Watcher::new(&fixture.0).unwrap();
        fs::write(&rule_file, "").unwrap();
        changed(&mut watcher);
        drain(&mut watcher);
        fs::write(&file, "after").unwrap();
        changed(&mut watcher);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn observes_ignored_file_force_added_after_watcher_starts() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join(".gitignore"), "ignored/\n").unwrap();
    fs::create_dir_all(fixture.0.join("ignored")).unwrap();
    let file = fixture.0.join("ignored/tracked.txt");
    fs::write(&file, "before").unwrap();
    let mut watcher = Watcher::new(&fixture.0).unwrap();
    assert!(Command::new("git")
        .args(["add", "-f", "ignored/tracked.txt"])
        .current_dir(&fixture.0)
        .status()
        .unwrap()
        .success());
    changed(&mut watcher);
    drain(&mut watcher);
    fs::write(file, "after").unwrap();
    changed(&mut watcher);
}

#[cfg(target_os = "linux")]
#[test]
fn observes_ignore_rules_after_git_info_is_recreated() {
    let fixture = Fixture::new();
    let info = fixture.0.join(".git/info");
    fs::remove_dir_all(&info).unwrap();
    fs::create_dir_all(fixture.0.join("ignored")).unwrap();
    let file = fixture.0.join("ignored/file.txt");
    fs::write(&file, "before").unwrap();
    let mut watcher = Watcher::new(&fixture.0).unwrap();
    fs::create_dir_all(&info).unwrap();
    fs::write(info.join("exclude"), "ignored/\n").unwrap();
    changed(&mut watcher);
    drain(&mut watcher);
    fs::write(info.join("exclude"), "").unwrap();
    changed(&mut watcher);
    drain(&mut watcher);
    fs::write(file, "after").unwrap();
    changed(&mut watcher);
}

#[cfg(windows)]
#[test]
fn ignores_uppercase_root_dark_generated_output() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join(".DARK")).unwrap();
    let mut watcher = Watcher::new(&fixture.0).unwrap();
    fs::write(fixture.0.join(".DARK/snapshot"), "generated").unwrap();
    let deadline = Instant::now() + Duration::from_millis(600);
    while Instant::now() < deadline {
        assert!(
            !watcher.take_changed().unwrap(),
            "root .DARK triggered refresh"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
