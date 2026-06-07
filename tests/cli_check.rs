#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use core::sync::atomic::{AtomicU64, Ordering};
use indoc::indoc;
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
        indoc! {r#"
            public class Good {}
        "#},
    );
    tempdir.add_file(
        "bad.java",
        indoc! {r#"
            public class bad {}
        "#},
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

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");

    assert!(!output.status.success());
    assert!(stdout.contains("lowercase-class"), "stdout: {stdout}");
    assert!(stdout.contains("bad"), "stdout: {stdout}");
    assert!(
        stderr.contains("Found 1 problems across 2 java files"),
        "stderr: {stderr}"
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
