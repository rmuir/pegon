#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use core::sync::atomic::{AtomicU64, Ordering};
use indoc::indoc;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn check_simple() {
    let tempdir = TempDir::new();
    tempdir.add_file(
        "Good.java",
        indoc! {"
            public class Good {}
        "},
    );
    tempdir.add_file(
        "bad.java",
        indoc! {"
            public class bad {}
        "},
    );

    let output = Command::new(env!("CARGO_BIN_EXE_pegon"))
        .args([
            "check",
            "--output-format",
            "concise",
            tempdir.0.to_str().expect("should be utf-8"),
        ])
        .output()
        .expect("run pegon");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");

    assert!(stdout.contains("lowercase-class"), "stdout: {stdout}");
    assert!(stdout.contains("bad"), "stdout: {stdout}");
    assert!(
        stderr.contains("Found 1 problems in 2 files"),
        "stderr: {stderr}"
    );
}

#[test]
fn check_fix() {
    let tempdir = TempDir::new();
    tempdir.add_file(
        "Fix.java",
        indoc! {"
            public class Fix {
                long one = 1l;
                long two = 2l;
            }
        "},
    );

    let output = Command::new(env!("CARGO_BIN_EXE_pegon"))
        .args([
            "check",
            "--fix",
            "--output-format",
            "concise",
            tempdir.0.to_str().expect("should be utf-8"),
        ])
        .output()
        .expect("run pegon");

    assert!(output.status.success());
    let contents = fs::read_to_string(tempdir.0.join("Fix.java")).expect("readable");
    assert_eq!(
        contents,
        indoc! {"
            public class Fix {
                long one = 1L;
                long two = 2L;
            }
        "}
    );
}

#[test]
fn analyze_simple() {
    let tempdir = TempDir::new();
    tempdir.add_file(
        "One.java",
        indoc! {"
            package my.package;

            public class One {}
        "},
    );

    tempdir.add_file(
        "Two.java",
        indoc! {"
            package my.package;

            public class Two {}
        "},
    );

    let output = Command::new(env!("CARGO_BIN_EXE_pegon"))
        .args(["analyze", tempdir.0.to_str().expect("should be utf-8")])
        .output()
        .expect("run pegon");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(output.status.success());
    let result: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        result,
        json!({
            "names": {
                "my.package.One": tempdir.0.join("One.java"),
                "my.package.Two": tempdir.0.join("Two.java")
            }
        })
    );
}

/// wild that rust has no way to do this
/// this isn't even quite right but we don't need perfection
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let name = format!(
            "{}-{}-{}-{}",
            env!("CARGO_PKG_NAME"),
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed),
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir_all(&path).expect("can create tempdir");
        Self(path)
    }

    fn add_file(&self, name: &str, contents: &str) {
        fs::write(self.0.join(name), contents).expect("write temp file");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);
