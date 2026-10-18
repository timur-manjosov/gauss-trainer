//! Tests of the compiled binary that do not need a terminal.

use std::process::{Command, Stdio};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gauss-trainer"))
}

#[test]
fn help_lists_the_options() {
    let out = binary().arg("--help").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Aufruf:") && text.contains("--seed") && text.contains("--exam"));
}

#[test]
fn version_is_printed() {
    let out = binary().arg("--version").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        text.trim(),
        format!("gauss-trainer {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn bad_arguments_fail_with_a_german_message() {
    let out = binary().args(["--size", "9x9"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.starts_with("Fehler: Ungültiger Wert »9x9« für --size"),
        "{err}"
    );

    let out = binary().arg("[[1,2],[3]]").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("Startmatrix")
    );
}

#[test]
fn refuses_to_start_without_a_terminal() {
    let out = binary()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("interaktives Terminal")
    );
}
