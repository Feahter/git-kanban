use crate::types::{Backend, GhIssue, Issue, IssueState, Priority};
use serde::{Deserialize, Serialize};
use std::process::Command;

// ── Glab-specific types ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GlabIssue {
    iid: u64,
    title: String,
    description: Option<String>,
    state: String,
    labels: Vec<String>,
    assignees: Vec<GlabAssignee>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GlabAssignee {
    username: String,
}

// ── Dispatch ────────────────────────────────────────────────

pub fn fetch_issues(backend: Backend, repo: &str) -> Result<Vec<Issue>, String> {
    match backend {
        Backend::GitHub => fetch_gh_issues(repo),
        Backend::GitLab => fetch_glab_issues(repo),
    }
}

pub fn check_cli_auth(backend: Backend) -> Result<String, String> {
    let cmd = backend.cmd();
    let output = Command::new(cmd)
        .args(["auth", "status"])
        .output()
        .map_err(|e| format!("{} not found: {}. Install from https://cli.github.com/ or https://gitlab.com/gitlab-org/cli", cmd, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{} not authenticated: {}", cmd, stderr.trim()));
    }
    Ok("ok".into())
}

pub fn create_issue(backend: Backend, repo: &str, title: &str, body: Option<&str>, labels: &[String]) -> Result<u64, String> {
    match backend {
        Backend::GitHub => create_gh_issue(repo, title, body, labels),
        Backend::GitLab => create_glab_issue(repo, title, body, labels),
    }
}

pub fn close_issue(backend: Backend, repo: &str, number: u64) -> Result<(), String> {
    let cmd = backend.cmd();
    run_cli_cmd(cmd, &["issue", "close", &number.to_string(), "--repo", repo])
}

pub fn move_issue(backend: Backend, repo: &str, number: u64, remove_labels: &[String], add_labels: &[String]) -> Result<(), String> {
    match backend {
        Backend::GitHub => {
            let mut args = vec![
                "issue".into(), "edit".into(), number.to_string(),
                "--repo".into(), repo.into(),
            ];
            if !remove_labels.is_empty() {
                args.push("--remove-label".into());
                args.push(remove_labels.join(","));
            }
            if !add_labels.is_empty() {
                args.push("--add-label".into());
                args.push(add_labels.join(","));
            }
            let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            run_cli_cmd("gh", &str_args)
        }
        Backend::GitLab => {
            if !remove_labels.is_empty() {
                run_cli_cmd("glab", &["issue", "update", &number.to_string(), "--repo", repo, "--unlabel", &remove_labels.join(",")])?;
            }
            if !add_labels.is_empty() {
                run_cli_cmd("glab", &["issue", "update", &number.to_string(), "--repo", repo, "--label", &add_labels.join(",")])?;
            }
            Ok(())
        }
    }
}

pub fn add_comment(backend: Backend, repo: &str, number: u64, body: &str) -> Result<(), String> {
    match backend {
        Backend::GitHub => run_cli_cmd("gh", &["issue", "comment", &number.to_string(), "--repo", repo, "--body", body]),
        Backend::GitLab => run_cli_cmd("glab", &["issue", "note", "-m", body, &number.to_string(), "--repo", repo]),
    }
}

pub fn assign_self(backend: Backend, repo: &str, number: u64) -> Result<(), String> {
    match backend {
        Backend::GitHub => run_cli_cmd("gh", &["issue", "edit", &number.to_string(), "--repo", repo, "--add-assignee", "@me"]),
        Backend::GitLab => {
            // Get current username via glab API, then assign
            let output = Command::new("glab")
                .args(["api", "/user"])
                .output()
                .map_err(|e| format!("Failed to run glab: {}", e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("glab api error: {}", stderr.trim()));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| format!("JSON parse error: {}", e))?;
            let username = v.get("username")
                .and_then(|u| u.as_str())
                .ok_or_else(|| "Could not determine GitLab username".to_string())?;
            run_cli_cmd("glab", &["issue", "update", &number.to_string(), "--repo", repo, "--assignee", username])
        }
    }
}

pub fn assign_user(backend: Backend, repo: &str, number: u64, username: &str) -> Result<(), String> {
    match backend {
        Backend::GitHub => run_cli_cmd("gh", &["issue", "edit", &number.to_string(), "--repo", repo, "--add-assignee", username]),
        Backend::GitLab => run_cli_cmd("glab", &["issue", "update", &number.to_string(), "--repo", repo, "--assignee", username]),
    }
}

pub fn reopen_issue(backend: Backend, repo: &str, number: u64) -> Result<(), String> {
    let cmd = backend.cmd();
    run_cli_cmd(cmd, &["issue", "reopen", &number.to_string(), "--repo", repo])
}

pub fn open_in_browser(backend: Backend, repo: &str, number: u64) {
    let url = match backend {
        Backend::GitHub => format!("https://github.com/{}/issues/{}", repo, number),
        Backend::GitLab => format!("https://gitlab.com/{}/-/issues/{}", repo, number),
    };
    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg(&url).output()
    } else if cfg!(target_os = "linux") {
        Command::new("xdg-open").arg(&url).output()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/c", "start", &url]).output()
    } else {
        eprintln!("Warning: cannot open browser — unsupported OS");
        return;
    };
    if let Err(e) = result {
        eprintln!("Warning: could not open browser: {}", e);
    }
}

pub fn edit_issue(backend: Backend, repo: &str, number: u64, title: Option<&str>, body: Option<&str>, add_labels: &[String], remove_labels: &[String]) -> Result<(), String> {
    match backend {
        Backend::GitHub => {
            let mut args = vec!["issue".to_string(), "edit".to_string(), number.to_string(), "--repo".to_string(), repo.to_string()];
            if let Some(t) = title { args.push("--title".into()); args.push(t.into()); }
            if let Some(b) = body { args.push("--body".into()); args.push(b.into()); }
            if !add_labels.is_empty() { args.push("--add-label".into()); args.push(add_labels.join(",")); }
            if !remove_labels.is_empty() { args.push("--remove-label".into()); args.push(remove_labels.join(",")); }
            let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            run_cli_cmd("gh", &str_args)
        }
        Backend::GitLab => {
            let mut args = vec!["issue".to_string(), "update".to_string(), number.to_string(), "--repo".to_string(), repo.to_string()];
            if let Some(t) = title { args.push("--title".into()); args.push(t.into()); }
            if let Some(b) = body { args.push("--description".into()); args.push(b.into()); }
            if !add_labels.is_empty() { args.push("--label".into()); args.push(add_labels.join(",")); }
            if !remove_labels.is_empty() { args.push("--unlabel".into()); args.push(remove_labels.join(",")); }
            let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            run_cli_cmd("glab", &str_args)
        }
    }
}

pub fn list_labels(backend: Backend, repo: &str) -> Result<Vec<String>, String> {
    match backend {
        Backend::GitHub => {
            let output = Command::new("gh")
                .args(["label", "list", "--repo", repo, "--json", "name", "--limit", "200"])
                .output()
                .map_err(|e| format!("Failed to run gh: {}", e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("gh error: {}", stderr.trim()));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            #[derive(serde::Deserialize)]
            struct GhLabel { name: String }
            let labels: Vec<GhLabel> = serde_json::from_str(&stdout)
                .map_err(|e| format!("JSON parse error: {}", e))?;
            Ok(labels.into_iter().map(|l| l.name).collect())
        }
        Backend::GitLab => {
            let output = Command::new("glab")
                .args(["label", "list", "--repo", repo])
                .output()
                .map_err(|e| format!("Failed to run glab: {}", e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("glab error: {}", stderr.trim()));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            // glab label list returns lines of "name (color)" — extract the name
            let labels: Vec<String> = stdout.lines()
                .filter_map(|line| line.split_once('(').map(|(name, _)| name.trim().to_string()))
                .filter(|name| !name.is_empty())
                .collect();
            Ok(labels)
        }
    }
}

// ── GitHub implementations ──────────────────────────────────

fn fetch_gh_issues(repo: &str) -> Result<Vec<Issue>, String> {
    let output = Command::new("gh")
        .args([
            "issue", "list",
            "--repo", repo,
            "--state", "all",
            "--json", "number,title,body,state,labels,assignees,createdAt,updatedAt",
            "--limit", "1000",
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

    let issues: Vec<Issue> = gh_issues
        .into_iter()
        .map(|gi| {
            let labels: Vec<String> = gi.labels.iter().map(|l| l.name.clone()).collect();
            let assignees: Vec<String> = gi.assignees.iter().map(|a| a.login.clone()).collect();
            let priority = Priority::from_labels(&labels);

            Issue {
                number: gi.number,
                title: gi.title,
                body: gi.body.unwrap_or_default(),
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
        })
        .collect();

    Ok(issues)
}

fn create_gh_issue(repo: &str, title: &str, body: Option<&str>, labels: &[String]) -> Result<u64, String> {
    let mut cmd = Command::new("gh");
    cmd.args(["issue", "create", "--repo", repo, "--title", title]);

    if let Some(b) = body {
        cmd.arg("--body");
        cmd.arg(b);
    }

    if !labels.is_empty() {
        cmd.arg("--label");
        cmd.arg(labels.join(","));
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to create issue: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Create failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_issue_number(&stdout)
}

// ── GitLab implementations ──────────────────────────────────

fn fetch_glab_issues(repo: &str) -> Result<Vec<Issue>, String> {
    let per_page = 100;
    let max_issues = 1000;
    let mut all_glab_issues: Vec<GlabIssue> = Vec::new();

    for page in 1.. {
        let output = Command::new("glab")
            .args([
                "issue", "list", "--repo", repo, "--all",
                "--output", "json",
                "--per-page", &per_page.to_string(),
                "--page", &page.to_string(),
            ])
            .output()
            .map_err(|e| format!("Failed to run glab: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("glab error: {}", stderr.trim()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let page_issues: Vec<GlabIssue> = serde_json::from_str(&stdout)
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let count = page_issues.len();
        all_glab_issues.extend(page_issues);

        // Stop if fewer than per_page — last page
        if count < per_page || all_glab_issues.len() >= max_issues {
            break;
        }
    }

    let issues: Vec<Issue> = all_glab_issues
        .into_iter()
        .map(|gi| {
            let assignees: Vec<String> = gi.assignees.iter().map(|a| a.username.clone()).collect();
            let priority = Priority::from_labels(&gi.labels);

            Issue {
                number: gi.iid,
                title: gi.title,
                body: gi.description.unwrap_or_default(),
                state: if gi.state == "closed" {
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
        })
        .collect();

    Ok(issues)
}

fn create_glab_issue(repo: &str, title: &str, body: Option<&str>, labels: &[String]) -> Result<u64, String> {
    let mut cmd = Command::new("glab");
    cmd.args(["issue", "create", "--repo", repo, "--title", title]);

    if let Some(b) = body {
        cmd.arg("--description");
        cmd.arg(b);
    }

    if !labels.is_empty() {
        cmd.arg("--label");
        cmd.arg(labels.join(","));
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to create issue: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Create failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_issue_number(&stdout)
}

// ── Helpers ─────────────────────────────────────────────────

fn parse_issue_number(stdout: &str) -> Result<u64, String> {
    if let Some(num_str) = stdout.trim().rsplit('/').next() {
        num_str
            .parse::<u64>()
            .map_err(|_| "Failed to parse issue number".into())
    } else {
        Err("Unexpected CLI output".into())
    }
}

fn run_cli_cmd(cmd: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", cmd, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{} error: {}", cmd, stderr.trim()));
    }
    Ok(())
}
