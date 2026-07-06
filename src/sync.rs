use std::process::Command;
use crate::types::{GhIssue, GlabIssue, Issue, IssueState, Platform, Priority};

/// Fetch issues from the configured platform.
pub fn fetch_issues(repo: &str, platform: &Platform) -> Result<Vec<Issue>, String> {
    match platform {
        Platform::Github => fetch_gh_issues(repo),
        Platform::Gitlab => fetch_glab_issues(repo),
    }
}

/// Check that the platform's CLI is installed and authenticated.
pub fn check_auth(platform: &Platform) -> Result<String, String> {
    let (cli, install_url) = match platform {
        Platform::Github => ("gh", "https://cli.github.com/"),
        Platform::Gitlab => ("glab", "https://gitlab.com/gitlab-org/cli"),
    };
    let output = Command::new(cli)
        .args(["auth", "status"])
        .output()
        .map_err(|e| format!("{cli} not found: {e}. Install from {install_url}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{cli} not authenticated: {}", stderr.trim()));
    }
    Ok("ok".into())
}

// ── GitHub ──

/// Fetch all issues (open + closed) from a GitHub repo via `gh issue list`.
fn fetch_gh_issues(repo: &str) -> Result<Vec<Issue>, String> {
    let output = Command::new("gh")
        .args([
            "issue", "list",
            "--repo", repo,
            "--state", "all",
            "--json", "number,title,state,labels,assignees,createdAt,updatedAt",
            "--limit", "200",
        ])
        .output()
        .map_err(|e| format!("Failed to run gh: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh error: {}", stderr.trim()));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_gh_response(&stdout)
}

/// Parse a `gh issue list --json` response string into `Issue` values.
pub fn parse_gh_response(json: &str) -> Result<Vec<Issue>, String> {
    let gh_issues: Vec<GhIssue> = serde_json::from_str(json)
        .map_err(|e| format!("JSON parse error: {e}"))?;
    Ok(gh_issues.into_iter().map(convert_gh_issue).collect())
}

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

// ── GitLab ──

/// Fetch all issues (open + closed) from a GitLab repo via `glab issue list`.
fn fetch_glab_issues(repo: &str) -> Result<Vec<Issue>, String> {
    let output = Command::new("glab")
        .args([
            "issue", "list",
            "--repo", repo,
            "--output", "json",
            "--per-page", "200",
        ])
        .output()
        .map_err(|e| format!("Failed to run glab: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("glab error: {}", stderr.trim()));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_glab_response(&stdout)
}

/// Parse a `glab issue list --output json` response string into `Issue` values.
pub fn parse_glab_response(json: &str) -> Result<Vec<Issue>, String> {
    let glab_issues: Vec<GlabIssue> = serde_json::from_str(json)
        .map_err(|e| format!("JSON parse error: {e}"))?;
    Ok(glab_issues.into_iter().map(convert_glab_issue).collect())
}

fn convert_glab_issue(gi: GlabIssue) -> Issue {
    let assignees: Vec<String> = gi.assignees.iter().map(|a| a.username.clone()).collect();
    let priority = Priority::from_labels(&gi.labels);
    Issue {
        number: gi.iid,
        title: gi.title,
        state: if gi.state == "closed" || gi.state == "merged" {
            IssueState::Closed
        } else {
            IssueState::Open
        },
        labels: gi.labels,
        assignees,
        priority,
        created_at: gi.created_at,
        updated_at: gi.updated_at,
    }
}

// ── Issue CRUD (GitHub only for now) ──

/// Create a new issue via `gh issue create`.
pub fn create_issue(repo: &str, title: &str, labels: &[String]) -> Result<u64, String> {
    let mut cmd = Command::new("gh");
    cmd.args(["issue", "create", "--repo", repo, "--title", title]);
    if !labels.is_empty() {
        cmd.arg("--label");
        cmd.arg(labels.join(","));
    }
    let output = cmd.output()
        .map_err(|e| format!("Failed to create issue: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Create failed: {}", stderr.trim()));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
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

/// Build the issue URL for a specific platform.
pub fn issue_url(repo: &str, number: u64, platform: &Platform) -> String {
    format!("https://{}/{}/issues/{}", platform.host(), repo, number)
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
    run_gh_cmd(&["issue", "edit", &number.to_string(), "--repo", repo, "--add-label", label])
}

/// Remove a label from an issue via `gh issue edit --remove-label`.
pub fn remove_label(repo: &str, number: u64, label: &str) -> Result<(), String> {
    run_gh_cmd(&["issue", "edit", &number.to_string(), "--repo", repo, "--remove-label", label])
}

fn run_gh_cmd(args: &[&str]) -> Result<(), String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run gh: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh error: {}", stderr.trim()));
    }
    Ok(())
}

/// Open an issue page in the default browser.
pub fn open_in_browser(repo: &str, number: u64, platform: &Platform) {
    let url = issue_url(repo, number, platform);
    let opener = if cfg!(target_os = "macos") { "open" }
                 else if cfg!(target_os = "linux") { "xdg-open" }
                 else { "open" }; // fallback
    Command::new(opener).arg(&url).output().ok();
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
    fn test_issue_url_github() {
        let url = issue_url("owner/repo", 42, &Platform::Github);
        assert_eq!(url, "https://github.com/owner/repo/issues/42");
    }

    #[test]
    fn test_issue_url_gitlab() {
        let url = issue_url("namespace/project", 7, &Platform::Gitlab);
        assert_eq!(url, "https://gitlab.com/namespace/project/issues/7");
    }

    // ── parse_gh_response ──

    fn sample_gh_response_json() -> &'static str {
        r#"[
  { "number": 1, "title": "Fix login bug", "state": "OPEN",
    "labels": [{"name":"bug"},{"name":"p0"}],
    "assignees": [{"login":"alice"},{"login":"bob"}],
    "createdAt": "2026-01-15T10:00:00Z", "updatedAt": "2026-06-01T12:00:00Z" },
  { "number": 42, "title": "Add dark mode", "state": "CLOSED",
    "labels": [{"name":"feature"},{"name":"p2"}], "assignees": [],
    "createdAt": "2026-03-01T08:00:00Z", "updatedAt": "2026-05-15T16:30:00Z" },
  { "number": 99, "title": "Merged PR", "state": "MERGED",
    "labels": [], "assignees": [{"login":"charlie"}],
    "createdAt": "2026-02-10T09:00:00Z", "updatedAt": "2026-02-11T09:00:00Z" }
]"#
    }

    #[test]
    fn test_parse_gh_response_counts() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        assert_eq!(issues.len(), 3);
    }

    #[test]
    fn test_parse_gh_response_open_state() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        assert_eq!(issues[0].state, IssueState::Open);
        assert_eq!(issues[0].title, "Fix login bug");
    }

    #[test]
    fn test_parse_gh_response_closed_state() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        assert_eq!(issues[1].state, IssueState::Closed);
    }

    #[test]
    fn test_parse_gh_response_merged_as_closed() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        assert_eq!(issues[2].state, IssueState::Closed);
    }

    #[test]
    fn test_parse_gh_response_labels() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        assert_eq!(issues[0].labels, vec!["bug", "p0"]);
    }

    #[test]
    fn test_parse_gh_response_priority() {
        let issues = parse_gh_response(sample_gh_response_json()).unwrap();
        assert_eq!(issues[0].priority, Some(Priority::P0));
        assert_eq!(issues[1].priority, Some(Priority::P2));
        assert_eq!(issues[2].priority, None);
    }

    #[test]
    fn test_parse_gh_response_empty() {
        let issues = parse_gh_response("[]").unwrap();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_parse_gh_response_invalid_json() {
        let err = parse_gh_response("not json").unwrap_err();
        assert!(err.contains("JSON parse error"));
    }

    // ── parse_glab_response ──

    fn sample_glab_response_json() -> &'static str {
        r#"[
  { "iid": 1, "title": "Fix login bug", "state": "opened",
    "labels": ["bug", "p0"],
    "assignees": [{"username":"alice"}],
    "created_at": "2026-01-15T10:00:00Z", "updated_at": "2026-06-01T12:00:00Z" },
  { "iid": 42, "title": "Add dark mode", "state": "closed",
    "labels": ["feature"], "assignees": [],
    "created_at": "2026-03-01T08:00:00Z", "updated_at": "2026-05-15T16:30:00Z" },
  { "iid": 99, "title": "Merged MR", "state": "merged",
    "labels": [], "assignees": [{"username":"charlie"}],
    "created_at": "2026-02-10T09:00:00Z", "updated_at": "2026-02-11T09:00:00Z" }
]"#
    }

    #[test]
    fn test_parse_glab_response_counts() {
        let issues = parse_glab_response(sample_glab_response_json()).unwrap();
        assert_eq!(issues.len(), 3);
    }

    #[test]
    fn test_parse_glab_response_open_state() {
        let issues = parse_glab_response(sample_glab_response_json()).unwrap();
        assert_eq!(issues[0].state, IssueState::Open);
        assert_eq!(issues[0].number, 1);
    }

    #[test]
    fn test_parse_glab_response_closed_state() {
        let issues = parse_glab_response(sample_glab_response_json()).unwrap();
        assert_eq!(issues[1].state, IssueState::Closed);
    }

    #[test]
    fn test_parse_glab_response_merged_as_closed() {
        let issues = parse_glab_response(sample_glab_response_json()).unwrap();
        assert_eq!(issues[2].state, IssueState::Closed);
    }

    #[test]
    fn test_parse_glab_response_labels_as_strings() {
        let issues = parse_glab_response(sample_glab_response_json()).unwrap();
        // GitLab returns labels as plain strings, not objects
        assert_eq!(issues[0].labels, vec!["bug", "p0"]);
    }

    #[test]
    fn test_parse_glab_response_priority() {
        let issues = parse_glab_response(sample_glab_response_json()).unwrap();
        assert_eq!(issues[0].priority, Some(Priority::P0));
    }

    #[test]
    fn test_parse_glab_response_empty() {
        let issues = parse_glab_response("[]").unwrap();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_parse_glab_response_invalid_json() {
        let err = parse_glab_response("bad").unwrap_err();
        assert!(err.contains("JSON parse error"));
    }
}
