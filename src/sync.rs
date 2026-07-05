use std::process::Command;
use crate::types::{GhIssue, Issue, IssueState, Priority};

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
    if let Some(num_str) = stdout.trim().rsplit('/').next() {
        num_str.parse::<u64>().map_err(|_| "Failed to parse issue number".into())
    } else {
        Err("Unexpected gh output".into())
    }
}

pub fn close_issue(repo: &str, number: u64) -> Result<(), String> {
    run_gh_cmd(&["issue", "close", &number.to_string(), "--repo", repo])
}

pub fn reopen_issue(repo: &str, number: u64) -> Result<(), String> {
    run_gh_cmd(&["issue", "reopen", &number.to_string(), "--repo", repo])
}

pub fn add_label(repo: &str, number: u64, label: &str) -> Result<(), String> {
    run_gh_cmd(&[
        "issue", "edit", &number.to_string(),
        "--repo", repo,
        "--add-label", label,
    ])
}

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

pub fn open_in_browser(repo: &str, number: u64) {
    let url = format!("https://github.com/{}/issues/{}", repo, number);
    // macOS `open` command
    Command::new("open").arg(&url).output().ok();
}
