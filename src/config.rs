use std::path::PathBuf;
use crate::types::Config;

const CONFIG_DIR: &str = "git-kanban";
const CONFIG_FILE: &str = "config.json";

fn config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".config")
        });
    base.join(CONFIG_DIR)
}

fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

fn cache_dir() -> PathBuf {
    let base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".cache")
        });
    base.join(CONFIG_DIR)
}

pub fn cache_file_path() -> PathBuf {
    cache_dir().join("issues.json")
}

pub fn load() -> Config {
    let path = config_path();

    if !path.exists() {
        // First run: create default config and exit
        std::fs::create_dir_all(config_dir()).ok();
        let default = Config::default();
        let json = serde_json::to_string_pretty(&serde_json::json!({
            "repo": "",
            "backend": "github",
            "note": "Set your repo in format 'owner/name'. Remove labels you don't use, add yours.",
            "columns": {
                "todo": ["todo", "status:todo"],
                "doing": ["doing", "status:doing", "in-progress"],
                "review": ["review", "status:review"],
                "done": ["done", "status:done"],
                "closed": []
            }
        })).unwrap();
        std::fs::write(&path, &json).ok();
        return default;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: cannot read config: {}", e);
            return Config::default();
        }
    };

    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Config::default(),
    };

    let mut cfg = Config::default();

    if let Some(repo) = parsed.get("repo").and_then(|v| v.as_str()) {
        if !repo.is_empty() {
            cfg.repo = repo.to_string();
        }
    }

    // Allow custom backend
    if let Some(backend) = parsed.get("backend").and_then(|v| v.as_str()) {
        cfg.backend = match backend {
            "gitlab" => crate::types::Backend::GitLab,
            _ => crate::types::Backend::GitHub,
        };
    }

    // Allow custom column labels via config
    if let Some(cols) = parsed.get("columns").and_then(|v| v.as_object()) {
        for col in cfg.columns.iter_mut() {
            if let Some(overrides) = cols.get(&col.id).and_then(|v| v.as_array()) {
                let custom_labels: Vec<String> = overrides
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if !custom_labels.is_empty() {
                    col.labels = custom_labels;
                }
            }
        }
    }

    cfg
}

pub fn write_cache(issues: &[crate::types::Issue], last_sync: &str) {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("issues.json");

    let data = serde_json::json!({
        "last_sync": last_sync,
        "issues": issues,
    });

    if let Ok(json) = serde_json::to_string_pretty(&data) {
        std::fs::write(&path, &json).ok();
    }
}

pub fn read_cache() -> Option<Vec<crate::types::Issue>> {
    let path = cache_file_path();
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let issues: Vec<crate::types::Issue> = serde_json::from_value(parsed.get("issues")?.clone()).ok()?;
    Some(issues)
}

// ── Unit tests ──
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Backend, Issue, IssueState, Priority};
    use std::sync::Mutex;

    /// Serialise cache tests under a global lock because env var
    /// manipulation is inherently racy.
    static CACHE_LOCK: Mutex<()> = Mutex::new(());

    fn make_test_issues() -> Vec<Issue> {
        vec![
            Issue {
                number: 1,
                title: "Test 1".into(),
                body: "Body 1".into(),
                state: IssueState::Open,
                labels: vec!["bug".into()],
                assignees: vec![],
                priority: Some(Priority::P1),
                created_at: "2024-01-01T00:00:00Z".into(),
                updated_at: "2024-01-02T00:00:00Z".into(),
            },
            Issue {
                number: 2,
                title: "Test 2".into(),
                body: String::new(),
                state: IssueState::Closed,
                labels: vec!["done".into()],
                assignees: vec!["user1".into()],
                priority: None,
                created_at: "2024-01-03T00:00:00Z".into(),
                updated_at: "2024-01-04T00:00:00Z".into(),
            },
        ]
    }

    #[test]
    fn test_cache_write_read_roundtrip() {
        let _lock = CACHE_LOCK.lock().unwrap();

        let tmp = std::env::temp_dir().join(format!("git-kanban-test-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        std::env::set_var("XDG_CACHE_HOME", &tmp);

        let issues = make_test_issues();
        write_cache(&issues, "2024-01-01T12:00:00Z");

        let path = cache_file_path();
        assert!(path.exists(), "cache file should exist");

        let loaded = read_cache();
        assert!(loaded.is_some(), "read_cache should return Some");
        let loaded = loaded.unwrap();
        assert_eq!(loaded.len(), 2);

        assert_eq!(loaded[0].number, 1);
        assert_eq!(loaded[0].title, "Test 1");
        assert_eq!(loaded[0].state, IssueState::Open);
        assert_eq!(loaded[0].labels, vec!["bug"]);

        assert_eq!(loaded[1].number, 2);
        assert_eq!(loaded[1].title, "Test 2");
        assert_eq!(loaded[1].state, IssueState::Closed);
        assert_eq!(loaded[1].assignees, vec!["user1"]);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_cache_no_file() {
        let _lock = CACHE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let tmp = std::env::temp_dir().join(format!("git-kanban-test-nocache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CACHE_HOME", &tmp);

        assert!(read_cache().is_none(), "should return None when no cache file");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_cache_corrupted() {
        let _lock = CACHE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let tmp = std::env::temp_dir().join(format!("git-kanban-test-corrupt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp.join("git-kanban")).unwrap();
        std::env::set_var("XDG_CACHE_HOME", &tmp);

        let path = cache_file_path();
        std::fs::write(&path, "not valid json").unwrap();

        assert!(read_cache().is_none(), "should return None for corrupt cache");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_creates_default_config() {
        let _lock = CACHE_LOCK.lock().unwrap();

        let tmp = std::env::temp_dir().join(format!("git-kanban-test-load-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        // First call should create default config
        let cfg = load();
        assert!(cfg.repo.is_empty(), "default repo should be empty");
        assert_eq!(cfg.backend, Backend::GitHub, "default backend should be GitHub");
        assert_eq!(cfg.columns.len(), 5, "default should have 5 columns");

        // Verify config file was created
        let path = config_path();
        assert!(path.exists(), "config file should have been created");

        // Second call should read the same config
        let cfg2 = load();
        assert!(cfg2.repo.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_config_columns_structure() {
        let _lock = CACHE_LOCK.lock().unwrap();

        let tmp = std::env::temp_dir().join(format!("git-kanban-test-cols-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let cfg = load();

        assert_eq!(cfg.columns[0].id, "todo");
        assert_eq!(cfg.columns[0].labels, vec!["todo", "status:todo"]);
        assert!(!cfg.columns[0].show_closed);

        assert_eq!(cfg.columns[1].id, "doing");
        assert_eq!(cfg.columns[1].labels, vec!["doing", "status:doing", "in-progress"]);

        assert_eq!(cfg.columns[4].id, "closed");
        assert!(cfg.columns[4].labels.is_empty());
        assert!(cfg.columns[4].show_closed);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_with_repo_set() {
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-repo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let config_dir = tmp.join("git-kanban");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.json"), r#"{"repo": "owner/repo"}"#).unwrap();

        let cfg = load();
        assert_eq!(cfg.repo, "owner/repo");
        assert_eq!(cfg.backend, Backend::GitHub);
        assert_eq!(cfg.columns.len(), 5);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_with_gitlab_backend() {
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-gl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let config_dir = tmp.join("git-kanban");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.json"), r#"{"repo": "group/project", "backend": "gitlab"}"#).unwrap();

        let cfg = load();
        assert_eq!(cfg.repo, "group/project");
        assert_eq!(cfg.backend, Backend::GitLab);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_with_custom_column_labels() {
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-custcols-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let config_dir = tmp.join("git-kanban");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.json"), r#"{"columns": {"todo": ["backlog", "triage"]}}"#).unwrap();

        let cfg = load();
        assert_eq!(cfg.columns[0].id, "todo");
        assert_eq!(cfg.columns[0].labels, vec!["backlog", "triage"]);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_cache_missing_issues_key() {
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-noissues-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp.join("git-kanban")).unwrap();
        std::env::set_var("XDG_CACHE_HOME", &tmp);

        let path = cache_file_path();
        std::fs::write(&path, r#"{"last_sync": "2024-01-01"}"#).unwrap();

        assert!(read_cache().is_none(), "should return None when issues key missing");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_invalid_backend_defaults_github() {
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-badbe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let config_dir = tmp.join("git-kanban");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.json"), r#"{"repo": "o/r", "backend": "invalid"}"#).unwrap();

        let cfg = load();
        assert_eq!(cfg.repo, "o/r");
        assert_eq!(cfg.backend, Backend::GitHub, "invalid backend should default to GitHub");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_empty_backend_defaults_github() {
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-emptybe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let config_dir = tmp.join("git-kanban");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.json"), r#"{"repo": "o/r", "backend": ""}"#).unwrap();

        let cfg = load();
        assert_eq!(cfg.repo, "o/r");
        assert_eq!(cfg.backend, Backend::GitHub, "empty backend string should default to GitHub");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_empty_repo_stays_empty() {
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-emptyrepo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let config_dir = tmp.join("git-kanban");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.json"), r#"{"repo": ""}"#).unwrap();

        let cfg = load();
        assert!(cfg.repo.is_empty(), "repo should stay empty when config has empty string");
        assert_eq!(cfg.backend, Backend::GitHub);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_empty_json_gets_all_defaults() {
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-emptyjson-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let config_dir = tmp.join("git-kanban");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.json"), r#"{}"#).unwrap();

        let cfg = load();
        assert!(cfg.repo.is_empty());
        assert_eq!(cfg.backend, Backend::GitHub);
        assert_eq!(cfg.columns.len(), 5);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_gitlab_backend_case_sensitive() {
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-glcaps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let config_dir = tmp.join("git-kanban");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.json"), r#"{"repo": "g/p", "backend": "GitLab"}"#).unwrap();

        let cfg = load();
        assert_eq!(cfg.repo, "g/p");
        // "GitLab" != "gitlab" (case sensitive match), should default to GitHub
        assert_eq!(cfg.backend, Backend::GitHub, "case-sensitive backend check: 'GitLab' should fall back to GitHub");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
