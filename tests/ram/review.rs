use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("git must be installed");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn fixture() -> Fixture {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("dark-review-ram-{}-{unique}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let fixture = Fixture(root);
    let root = &fixture.0;
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "Dark Test"]);
    git(root, &["config", "user.email", "dark-test@example.invalid"]);
    for file in 0..30 {
        let text: String = (0..100)
            .map(|line| format!("old value {file}:{line}\n"))
            .collect();
        fs::write(root.join(format!("file_{file:02}.txt")), text).unwrap();
    }
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "Fixture baseline"]);
    for file in 0..30 {
        let text: String = (0..100)
            .map(|line| {
                format!(
                    "{} value {file}:{line}\n",
                    if line < 50 { "new" } else { "old" }
                )
            })
            .collect();
        fs::write(root.join(format!("file_{file:02}.txt")), text).unwrap();
    }
    git(root, &["add", "file_00.txt", "file_01.txt"]);
    fixture
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "RAM budget measures release binary; run cargo test --release"
)]
fn release_review_fixture_meets_real_memory_limits() {
    let fixture = fixture();
    let output = Command::new(env!("CARGO_BIN_EXE_dark"))
        .arg("--self-test-review")
        .current_dir(&fixture.0)
        .output()
        .expect("could not start review self-test");
    assert!(
        output.status.success(),
        "review self-test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = String::from_utf8(output.stdout).expect("report is not UTF-8");
    let field = |key: &str| {
        report
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .unwrap_or_else(|| panic!("missing {key}: {report}"))
    };
    let expected = if cfg!(windows) {
        "private_working_set"
    } else {
        "rss"
    };
    assert_eq!(
        field("memory_metric="),
        expected,
        "wrong memory metric: {report}"
    );
    let files: usize = field("review_files=").parse().expect("invalid file count");
    let idle: u64 = field("idle_bytes=").parse().expect("invalid idle bytes");
    let peak: u64 = field("peak_sampled_bytes=")
        .parse()
        .expect("invalid peak bytes");
    assert!(files >= 30, "fixture was not fully reviewed: {report}");
    assert!(idle > 0 && peak >= idle, "invalid samples: {report}");
    assert!(idle < 2_000_000, "review idle RAM exceeded 2 MB: {report}");
    assert!(
        peak < 10_000_000,
        "review sampled peak RAM exceeded 10 MB: {report}"
    );
    println!("{report}");
}
