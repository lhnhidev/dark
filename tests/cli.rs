use std::process::Command;

#[test]
fn rejects_unknown_argument_instead_of_opening_the_terminal() {
    let output = Command::new(env!("CARGO_BIN_EXE_dark"))
        .arg("--not-a-dark-option")
        .output()
        .expect("could not start dark");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown argument"));
}
