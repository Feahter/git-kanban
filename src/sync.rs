use std::process::Command;
use crate::types::{GhIssue, Issue, IssueState, Priority};

/// Fetch all issues (open + closed) from a GitHub repo via `gh issue list`.
///
/// Returns up to 200 issues with their labels and assignees already resolved
/// into string lists. Requires `gh` to be installed and authenticated.
pub fn fetch_issues(repo: &str) -> Result<Vec<Issue>, String> {
    let output = Command::new("gh")
        .args([
            "issue", "list",
            "--repo", repo,
            "--state", "all",
            "--json", "number,title,state,labels,assignees,createdAt,updatedAt",
            "--limit", "200",
        ])
        .output()
        .map_err(|e| format!("Failed to run gh: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh error: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_gh_response(&stdout)
}

/// Parse a `gh issue list --json` response string into `Issue` values.
///
/// Separated from `fetch_issues` so it can be unit-tested without a real `gh` call.
pub fn parse_gh_response(json: &str) -> Result<Vec<Issue>, String> {
    let gh_issues: Vec<GhIssue> = serde_json::from_str(json)
        .map_err(|e| format!("JSON parse error: {e}"))?;
    Ok(gh_issues.into_iter().map(convert_gh_issue).collect())
}

/// Convert a raw `GhIssue` (from GitHub API) into a processed `Issue`.
fn convert_gh_issue(gi: GhIssue) -> Issue {
    let labels: Vec<String> = gi.labels.iter().map(|l| l.name.clone()).collect();
    let assignees: Vec<String> = gi.assignees.iter().map(|a| a.login.clone()).collect();
    let priority = Priority::from_labels(&labels);
    Issue {
        number: gi.number,
        title: gi.title,
        state: if gi.state == "CLOSED" || gi.state == "MERGED" {
            IssueState::Closed
        } else {
            IssueState::Open
        },
        labels,
        assignees,
        priority,
        created_at: gi.created_at,
        updated_at: gi.updated_at,
    }
}

/// Check that `gh` is installed and authenticated.
///
/// Returns `"ok"` on success, or an error message describing the issue.
pub fn check_gh_auth() -> Result<String, String> {
    let output = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map_err(|e| format!("gh not found: {}. Install from https://cli.github.com/", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh not authenticated: {}", stderr.trim()));
    }
    Ok("ok".into())
}

/// Create a new issue via `gh issue create`.
///
/// Returns the new issue number on success. The issue inherits the first
/// label from `labels` if any are provided.
pub fn create_issue(repo: &str, title: &str, labels: &[String]) -> Result<u64, String> {
    let mut cmd = Command::new("gh");
    cmd.args(["issue", "create", "--repo", repo, "--title", title]);

    if !labels.is_empty() {
        cmd.arg("--label");
        cmd.arg(labels.join(","));
    }

    let output = cmd.output()
        .map_err(|e| format!("Failed to create issue: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Create failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // gh returns URL like https://github.com/owner/repo/issues/123
    parse_issue_url(stdout.trim())
}

/// Extract the issue number from a `gh issue create` URL.
///
/// URL format: `https://github.com/owner/repo/issues/123`
pub fn parse_issue_url(url: &str) -> Result<u64, String> {
    if let Some(num_str) = url.trim().rsplit('/').next() {
        num_str.parse::<u64>().map_err(|_| format!("Failed to parse issue number from: {url}"))
    } else {
        Err(format!("Unexpected gh output: {url}"))
    }
}

/// Build the GitHub URL for an issue (without opening the browser).
pub fn issue_url(repo: &str, number: u64) -> String {
    format!("https://github.com/{}/issues/{}", repo, number)
}

/// Close an issue via `gh issue close`.
pub fn close_issue(repo: &str, number: u64) -> Result<(), String> {
    run_gh_cmd(&["issue", "close", &number.to_string(), "--repo", repo])
}

/// Reopen a closed issue via `gh issue reopen`.
pub fn reopen_issue(repo: &str, number: u64) -> Result<(), String> {
    run_gh_cmd(&["issue", "reopen", &number.to_string(), "--repo", repo])
}

/// Add a label to an issue via `gh issue edit --add-label`.
pub fn add_label(repo: &str, number: u64, label: &str) -> Result<(), String> {
    run_gh_cmd(&[
        "issue", "edit", &number.to_string(),
        "--repo", repo,
        "--add-label", label,
    ])
}

/// Remove a label from an issue via `gh issue edit --remove-label`.
pub fn remove_label(repo: &str, number: u64, label: &str) -> Result<(), String> {
    run_gh_cmd(&[
        "issue", "edit", &number.to_string(),
        "--repo", repo,
        "--remove-label", label,
    ])
}

fn run_gh_cmd(args: &[&str]) -> Result<(), String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run gh: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh error: {}", stderr.trim()));
    }
    Ok(())
}

/// Open an issue page in the default browser (macOS `open` command).
pub fn open_in_browser(repo: &str, number: u64) {
    Command::new("open").arg(&issue_url(repo, number)).output().ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_issue_url / issue_url ──

    #[test]
    fn test_parse_issue_url_normal() {
        let url = "https://github.com/owner/repo/issues/42";
        assert_eq!(parse_issue_url(url), Ok(42));
    }

    #[test]
    fn test_parse_issue_url_with_newline() {
        let url = "https://github.com/owner/repo/issues/123\n";
        assert_eq!(parse_issue_url(url), Ok(123));
    }

    #[test]
    fn test_parse_issue_url_invalid() {
        assert!(parse_issue_url("not-a-url").is_err());
    }

    #[test]
    fn test_parse_issue_url_empty() {
        assert!(parse_issue_url("").is_err());
    }

    #[test]
    fn test_issue_url_format() {
        let url = issue_url("owner/repo", 42);
        assert_eq!(url, "https://github.com/owner/repo/issues/42");
    }

    // ── parse_gh_response ──

    /// A realistic gh issue list JSON response for testing.
    fn sample_gh_response_json() -> &'static str {
        r#"[
  {
    "number": 1,
    "title": "Fix login bug",
    "state": "OPEN",
    "labels": [{"name": "bug"}, {"name": "p0"}],
    "assignees": [{"login": "alice"}, {"login": "bob"}],
    "createdAt": "2026-01-15T10:00:00Z",
    "updatedAt": "2026-06-01T12:00:00Z"
  },
  {
    "number": 42,
    "title": "Add dark mode",
    "state": "CLOSED",
    "labels": [{"name": "feature"}, {"name": "p2"}],
    "assignees": [],
    "createdAt": "2026-03-01T08:00:00Z",
    "updatedAt": "2026-05-15T16:30:00Z"
  },
  {
    "number": 99,
    "title": "Merged PR",
    "state": "MERGED",
    "labels": [],
    "assignees": [{"login": "charlie"}],
    "createdAt": "2026-02-10T09:00:00Z",
    "updatedAt": "2026-02-11T09:00:00Z"
  }
]"#
    }

    #[test]
    fn test_parse_gh_response_counts() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        assert_eq!(issues.len(), 3, "should parse 3 issues");
    }

    #[test]
    fn test_parse_gh_response_open_state() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        let issue = &issues[0];
        assert_eq!(issue.number, 1);
        assert_eq!(issue.title, "Fix login bug");
        assert_eq!(issue.state, IssueState::Open);
    }

    #[test]
    fn test_parse_gh_response_closed_state() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        let issue = &issues[1];
        assert_eq!(issue.number, 42);
        assert_eq!(issue.state, IssueState::Closed);
        assert_eq!(issue.title, "Add dark mode");
    }

    #[test]
    fn test_parse_gh_response_merged_as_closed() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        // MERGED should map to IssueState::Closed
        assert_eq!(issues[2].state, IssueState::Closed);
    }

    #[test]
    fn test_parse_gh_response_labels_and_assignees() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        // Issue 1: labels = ["bug", "p0"], assignees = ["alice", "bob"]
        assert_eq!(issues[0].labels, vec!["bug".to_string(), "p0".to_string()]);
        assert_eq!(issues[0].assignees, vec!["alice".to_string(), "bob".to_string()]);
        // Issue 42: no assignees
        assert!(issues[1].assignees.is_empty());
        // Issue 99: empty labels
        assert!(issues[2].labels.is_empty());
    }

    #[test]
    fn test_parse_gh_response_priority_from_labels() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        assert_eq!(issues[0].priority, Some(Priority::P0));
        assert_eq!(issues[1].priority, Some(Priority::P2));
        assert_eq!(issues[2].priority, None);
    }

    #[test]
    fn test_parse_gh_response_timestamps() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        assert_eq!(issues[0].created_at, "2026-01-15T10:00:00Z");
        assert_eq!(issues[0].updated_at, "2026-06-01T12:00:00Z");
    }

    #[test]
    fn test_parse_gh_response_invalid_json() {
        let result = parse_gh_response("not json at all");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("JSON parse error"), "error should mention JSON: {err}");
    }

    #[test]
    fn test_parse_gh_response_missing_field() {
        let result = parse_gh_response(r#"[{"number": 1}]"#);
        // Missing required fields cause serde errors
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_gh_response_empty_array() {
        let issues = parse_gh_response("[]").unwrap();
        assert!(issues.is_empty());
    }
}
