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
    let gh_issues: Vec<GhIssue> = serde_json::from_str(&stdout)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let issues: Vec<Issue> = gh_issues.into_iter().map(|gi| {
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
    }).collect();

    Ok(issues)
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
}
