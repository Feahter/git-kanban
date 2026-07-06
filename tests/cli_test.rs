/// Integration tests for gh-kanban CLI.
///
/// Tests are split into two groups:
/// 1. Synthetic tests: compile and run the binary, test CLI argument parsing
///    and error handling. These do NOT require GitHub authentication.
/// 2. Live JSON tests: run against a real GitHub repo via `gh`. These require
///    `gh` to be installed and authenticated (skipped if not available).

use std::process::Command;

fn binary_path() -> String {
    let cargo_manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set (run via `cargo test`)");
    format!("{}/target/debug/gh-kanban", cargo_manifest)
}

// ── Synthetic tests (no auth needed) ──

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

// ── Live JSON tests (require `gh` auth) ──

const TEST_REPO: &str = "Feahter/gh-kanban";

/// Check if `gh` is authenticated enough for tests.
fn gh_available() -> bool {
    Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn test_json_output_structure() {
    if !gh_available() {
        eprintln!("skipping: gh not available");
        return;
    }
    let output = Command::new(binary_path())
        .args(["--json", "--repo", TEST_REPO])
        .output()
        .expect("failed to execute binary");
    assert!(output.status.success(), "exit: {}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout should be valid JSON");
    assert!(parsed.get("repo").is_some(), "JSON should have 'repo' field");
    let issues = parsed["issues"]
        .as_array()
        .expect("'issues' should be an array");
    // The repo may have zero issues — structure is what matters
    for issue in issues {
        let obj = issue.as_object().expect("each issue should be an object");
        assert!(obj.contains_key("number"), "issue missing 'number': {obj:?}");
        assert!(obj.contains_key("title"), "issue missing 'title'");
        assert!(obj.contains_key("state"), "issue missing 'state'");
        assert!(obj.contains_key("labels"), "issue missing 'labels'");
        assert!(obj.contains_key("assignees"), "issue missing 'assignees'");
    }
}

#[test]
fn test_json_column_filter() {
    if !gh_available() {
        eprintln!("skipping: gh not available");
        return;
    }
    // Fetch full list and column-filtered list to compare counts
    let full = run_json(&[TEST_REPO]);
    let filtered = run_json(&[TEST_REPO, "--column", "done"]);
    // Filtered should have fewer or equal issues
    let full_count = full["count"].as_u64().unwrap();
    let filtered_count = filtered["count"].as_u64().unwrap();
    assert!(filtered_count <= full_count,
        "filtered ({filtered_count}) should be <= full ({full_count})");
    // All filtered issues should have "done" or "status:done" label
    for issue in filtered["issues"].as_array().unwrap() {
        let labels = issue["labels"].as_array().unwrap();
        let label_strs: Vec<&str> = labels.iter()
            .filter_map(|l| l.as_str())
            .collect();
        assert!(
            label_strs.contains(&"done") || label_strs.contains(&"status:done"),
            "filtered issue has no 'done' label: {issue:?}"
        );
    }
}

#[test]
fn test_json_column_filter_all() {
    if !gh_available() {
        eprintln!("skipping: gh not available");
        return;
    }
    // Filter by each default column and verify returns
    for col in &["todo", "doing", "review", "done"] {
        let result = run_json(&[TEST_REPO, "--column", col]);
        assert!(
            result["issues"].as_array().unwrap().len() > 0
                || result["count"].as_u64().unwrap() == 0,
            "column '{col}' should not produce errors"
        );
    }
}

#[test]
fn test_json_fields_filter() {
    if !gh_available() {
        eprintln!("skipping: gh not available");
        return;
    }
    let result = run_json(&[TEST_REPO, "--fields", "number,title,state"]);
    for issue in result["issues"].as_array().unwrap() {
        let obj = issue.as_object().unwrap();
        // Should only have the requested fields (plus maybe more if top-level)
        // Actually select_fields runs recursively, so nested objects keep only matching.
        assert!(obj.contains_key("number"), "missing number");
        assert!(obj.contains_key("title"), "missing title");
        assert!(obj.contains_key("state"), "missing state");
        // Should NOT have extra fields
        assert!(!obj.contains_key("labels"), "should not have 'labels': {obj:?}");
        assert!(!obj.contains_key("assignees"), "should not have 'assignees'");
        assert!(!obj.contains_key("priority"), "should not have 'priority'");
    }
}

#[test]
fn test_json_fields_single() {
    if !gh_available() {
        eprintln!("skipping: gh not available");
        return;
    }
    let result = run_json(&[TEST_REPO, "--fields", "number"]);
    for issue in result["issues"].as_array().unwrap() {
        let obj = issue.as_object().unwrap();
        assert_eq!(obj.len(), 1, "should only have 'number': {obj:?}");
        assert!(obj.contains_key("number"));
    }
}

#[test]
fn test_summary_output_structure() {
    if !gh_available() {
        eprintln!("skipping: gh not available");
        return;
    }
    let output = Command::new(binary_path())
        .args(["--summary", "--repo", TEST_REPO])
        .output()
        .expect("failed to execute binary");
    assert!(output.status.success(), "exit: {}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout should be valid JSON");
    assert!(parsed.get("repo").is_some(), "summary should have 'repo'");
    assert!(parsed.get("total").is_some(), "summary should have 'total'");
    assert!(parsed.get("columns").is_some(), "summary should have 'columns'");
    let columns = parsed["columns"].as_array().expect("'columns' should be an array");
    assert_eq!(columns.len(), 5, "should have 5 columns");
    for col in columns {
        let obj = col.as_object().expect("each column should be an object");
        assert!(obj.contains_key("id"), "column missing 'id': {obj:?}");
        assert!(obj.contains_key("title"), "column missing 'title'");
        assert!(obj.contains_key("count"), "column missing 'count'");
    }
    // Total should equal sum of column counts
    let col_sum: u64 = columns.iter()
        .map(|c| c["count"].as_u64().unwrap())
        .sum();
    assert_eq!(parsed["total"].as_u64().unwrap(), col_sum,
        "total should equal sum of column counts");
}

#[test]
fn test_summary_column_filter() {
    if !gh_available() {
        eprintln!("skipping: gh not available");
        return;
    }
    let full = run_json_summary(&[TEST_REPO]);
    let filtered = run_json_summary(&[TEST_REPO, "--column", "doing"]);
    // Filtered should return only 1 column
    assert_eq!(filtered["columns"].as_array().unwrap().len(), 1,
        "filtered summary should have 1 column");
    assert_eq!(filtered["columns"][0]["id"], "doing");
    // Total should equal the filtered column's count
    assert_eq!(filtered["total"].as_u64().unwrap(),
        filtered["columns"][0]["count"].as_u64().unwrap());
    // The single column's count should match the full summary's doing count
    let doing_count = full["columns"].as_array().unwrap().iter()
        .find(|c| c["id"] == "doing")
        .map(|c| c["count"].as_u64().unwrap())
        .unwrap_or(0);
    assert_eq!(filtered["columns"][0]["count"].as_u64().unwrap(), doing_count);
}

#[test]
fn test_refresh_output() {
    if !gh_available() {
        eprintln!("skipping: gh not available");
        return;
    }
    let output = Command::new(binary_path())
        .args(["--refresh", "--repo", TEST_REPO])
        .output()
        .expect("failed to execute binary");
    assert!(output.status.success(), "exit: {}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cached"), "refresh should say 'Cached': {stdout}");
    assert!(stdout.contains(TEST_REPO), "refresh should mention repo: {stdout}");
    // Should mention a number
    assert!(stdout.chars().any(|c| c.is_ascii_digit()), "refresh should have a count: {stdout}");
}

// ── Helpers ──

/// Run `gh-kanban --json --repo X [--extra args]` and parse JSON.
fn run_json(args: &[&str]) -> serde_json::Value {
    let mut cmd = Command::new(binary_path());
    cmd.arg("--json").arg("--repo");
    for a in args {
        cmd.arg(a);
    }
    let output = cmd.output().expect("failed to execute binary");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("gh-kanban --json failed: {stderr}");
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).expect("stdout should be valid JSON")
}

/// Run `gh-kanban --summary --repo X [--extra args]` and parse JSON.
fn run_json_summary(args: &[&str]) -> serde_json::Value {
    let mut cmd = Command::new(binary_path());
    cmd.arg("--summary").arg("--repo");
    for a in args {
        cmd.arg(a);
    }
    let output = cmd.output().expect("failed to execute binary");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("gh-kanban --summary failed: {stderr}");
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).expect("stdout should be valid JSON")
}
