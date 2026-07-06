/// Integration tests for gh-kanban CLI.
///
/// These tests compile and run the binary, testing CLI argument parsing
/// and error handling. They do NOT require GitHub authentication.

use std::process::Command;

fn binary_path() -> String {
    let cargo_manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set (run via `cargo test`)");
    format!("{}/target/debug/gh-kanban", cargo_manifest)
}

#[test]
fn test_help_exits_ok() {
    let output = Command::new(binary_path())
        .arg("--help")
        .output()
        .expect("failed to execute binary");
    assert!(output.status.success(), "exit code: {}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("gh-kanban"), "help should mention gh-kanban");
    assert!(stdout.contains("--json"), "help should mention --json");
    assert!(stdout.contains("--repo"), "help should mention --repo");
}

#[test]
fn test_no_repo_exits_with_error() {
    let output = Command::new(binary_path())
        .output()
        .expect("failed to execute binary");
    assert!(!output.status.success(), "should exit with error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no repository"), "stderr: {stderr}");
}

#[test]
fn test_json_without_repo_exits_with_error() {
    let output = Command::new(binary_path())
        .arg("--json")
        .output()
        .expect("failed to execute binary");
    assert!(!output.status.success(), "should exit with error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no repository"), "stderr: {stderr}");
}

#[test]
fn test_refresh_without_repo_exits_with_error() {
    let output = Command::new(binary_path())
        .arg("--refresh")
        .output()
        .expect("failed to execute binary");
    assert!(!output.status.success(), "should exit with error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no repository"), "stderr: {stderr}");
}

#[test]
fn test_summary_without_repo_exits_with_error() {
    let output = Command::new(binary_path())
        .arg("--summary")
        .output()
        .expect("failed to execute binary");
    assert!(!output.status.success(), "should exit with error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no repository"), "stderr: {stderr}");
}

#[test]
fn test_unknown_flag_helpful_error() {
    let output = Command::new(binary_path())
        .arg("--nonsense")
        .output()
        .expect("failed to execute binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--nonsense") || stderr.contains("error"), "stderr: {stderr}");
}

#[test]
fn test_binary_name_in_help() {
    let output = Command::new(binary_path())
        .arg("--help")
        .output()
        .expect("failed to execute binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("gh-kanban"));
    assert!(stdout.contains("kanban"));
    assert!(stdout.contains("GitHub"));
}

#[test]
fn test_help_mentions_new_flags() {
    let output = Command::new(binary_path())
        .arg("--help")
        .output()
        .expect("failed to execute binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--column"), "help should mention --column");
    assert!(stdout.contains("--fields"), "help should mention --fields");
    assert!(stdout.contains("--summary"), "help should mention --summary");
}
