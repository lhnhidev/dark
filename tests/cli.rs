use std::process::{Command, Output, Stdio};

fn run(argument: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dark"))
        .arg(argument)
        .stdin(Stdio::null())
        .output()
        .expect("could not start dark")
}

fn plain_success(output: &Output) -> &str {
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(
        !output.stdout.contains(&0x1b),
        "unexpected terminal control sequence"
    );
    std::str::from_utf8(&output.stdout).expect("output is not UTF-8")
}

#[test]
fn help_describes_user_options_and_keys_without_a_terminal() {
    for argument in ["--help", "-h"] {
        let output = run(argument);
        let text = plain_success(&output);
        for expected in [
            "--help",
            "--version",
            "--debug",
            "1",
            "2",
            "3",
            "q",
            "Ctrl-C",
        ] {
            assert!(text.contains(expected), "help missing {expected:?}: {text}");
        }
    }
}

#[test]
fn version_uses_package_version_without_a_terminal() {
    for argument in ["--version", "-V"] {
        let output = run(argument);
        assert_eq!(
            plain_success(&output).trim(),
            format!("dark {}", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn rejects_unknown_argument_instead_of_opening_the_terminal() {
    let output = run("--not-a-dark-option");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.contains(&0x1b));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--not-a-dark-option"));
}
