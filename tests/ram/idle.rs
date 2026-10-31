use std::process::Command;

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "RAM budget measures release binary; run cargo test --release"
)]
fn release_binary_stays_within_memory_budget_after_navigation() {
    let output = Command::new(env!("CARGO_BIN_EXE_dark"))
        .arg("--self-test")
        .output()
        .expect("could not start dark self-test");
    assert!(
        output.status.success(),
        "self-test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = String::from_utf8(output.stdout).expect("self-test report is not UTF-8");
    let value = |key: &str| {
        report
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .unwrap_or_else(|| panic!("missing {key} in self-test report: {report}"))
    };
    let metric = value("memory_metric=");
    assert!(
        matches!(metric, "private_working_set" | "rss"),
        "memory measurement unavailable or unknown ({metric}); cannot verify RAM budget"
    );
    let idle: u64 = value("idle_bytes=").parse().expect("invalid idle_bytes");
    let peak: u64 = value("peak_sampled_bytes=")
        .parse()
        .expect("invalid peak_sampled_bytes");
    assert_eq!(value("screens="), "3", "not all screens were exercised");
    assert!(idle > 0 && peak >= idle, "invalid samples: {report}");
    assert!(idle < 2_000_000, "idle RAM exceeded 2 MB: {report}");
    assert!(
        peak < 10_000_000,
        "sampled peak RAM exceeded 10 MB: {report}"
    );
    println!("{report}");
}
