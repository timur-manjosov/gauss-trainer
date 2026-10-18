//! The demo screen in the README must be the real output of the program.

use std::fs;
use std::path::Path;

/// Strips the insta front matter and the trailing newline from a snapshot file.
fn snapshot_body(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name);
    let raw = fs::read_to_string(path).expect("snapshot file");
    let mut parts = raw.splitn(3, "---\n");
    parts.next();
    parts.next();
    parts.next().expect("snapshot body").trim_end().to_owned()
}

#[test]
fn readme_embeds_the_current_demo_screen() {
    let readme = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("README.md");
    let demo = snapshot_body("snapshots__readme_demo.snap");
    assert!(
        readme.contains(&demo),
        "README.md does not contain the current demo screen. Re-run the snapshot tests and paste \
         tests/snapshots/snapshots__readme_demo.snap into the README."
    );
}

#[test]
fn readme_mentions_every_cli_option() {
    let readme = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("README.md");
    for option in [
        "--seed",
        "--size",
        "--preset",
        "--matrix",
        "--theme",
        "--exam",
        "--no-live",
        "--colors",
        "--simple-brackets",
    ] {
        assert!(
            readme.contains(option),
            "{option} is not documented in the README"
        );
    }
}
