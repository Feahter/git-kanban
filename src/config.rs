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

pub fn cache_file_path(repo: &str) -> PathBuf {
    let safe_name = repo.replace('/', "-");
    cache_dir().join(format!("issues-{}.json", safe_name))
}

pub fn load() -> Config {
    let path = config_path();

    if !path.exists() {
        return Config::default();
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
        Err(e) => {
            eprintln!("Warning: failed to parse config.json ({}), using defaults", e);
            return Config::default();
        }
    };

    let mut cfg = Config::default();

    if let Some(repo) = parsed.get("repo").and_then(|v| v.as_str()) {
        if !repo.is_empty() {
            cfg.repo = repo.to_string();
        }
    }

    // Load repos array (takes priority over single repo)
    if let Some(repos) = parsed.get("repos").and_then(|v| v.as_array()) {
        let list: Vec<String> = repos.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .filter(|s| !s.is_empty())
            .collect();
        if !list.is_empty() {
            cfg.repos = list;
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

/// Write cache with atomic rename to prevent partial writes.
/// Returns `true` on success, `false` on failure (caller should log).
pub fn write_cache(issues: &[crate::types::Issue], last_sync: &str, repo: &str) -> bool {
    let dir = cache_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return false;
    }
    let safe_name = repo.replace('/', "-");
    let path = dir.join(format!("issues-{}.json", safe_name));
    let tmp_path = dir.join(format!("issues-{}.tmp", safe_name));

    let data = serde_json::json!({
        "last_sync": last_sync,
        "repo": repo,
        "issues": issues,
    });

    let json = match serde_json::to_string_pretty(&data) {
        Ok(j) => j,
        Err(_) => return false,
    };

    if std::fs::write(&tmp_path, &json).is_err() {
        return false;
    }
    std::fs::rename(&tmp_path, &path).is_ok()
}

pub fn read_cache(repo: &str) -> Option<Vec<crate::types::Issue>> {
    let path = cache_file_path(repo);
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let issues: Vec<crate::types::Issue> = serde_json::from_value(parsed.get("issues")?.clone()).ok()?;
    Some(issues)
}

/// Read cache and return (issues, last_sync) metadata.
pub fn read_cache_meta(repo: &str) -> Option<(Vec<crate::types::Issue>, String)> {
    let path = cache_file_path(repo);
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let issues: Vec<crate::types::Issue> = serde_json::from_value(parsed.get("issues")?.clone()).ok()?;
    let last_sync: String = parsed.get("last_sync")?.as_str()?.to_string();
    Some((issues, last_sync))
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
        write_cache(&issues, "2024-01-01T12:00:00Z", "test/repo");

        let path = cache_file_path("test/repo");
        assert!(path.exists(), "cache file should exist");

        let loaded = read_cache("test/repo");
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

        assert!(read_cache("test/repo").is_none(), "should return None when no cache file");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_cache_corrupted() {
        let _lock = CACHE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let tmp = std::env::temp_dir().join(format!("git-kanban-test-corrupt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp.join("git-kanban")).unwrap();
        std::env::set_var("XDG_CACHE_HOME", &tmp);

        let path = cache_file_path("test/repo");
        std::fs::write(&path, "not valid json").unwrap();

        assert!(read_cache("test/repo").is_none(), "should return None for corrupt cache");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_creates_default_config() {
        let _lock = CACHE_LOCK.lock().unwrap();

        let tmp = std::env::temp_dir().join(format!("git-kanban-test-load-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        // First call should return defaults without creating a file
        let cfg = load();
        assert!(cfg.repo.is_empty(), "default repo should be empty");
        assert_eq!(cfg.backend, Backend::GitHub, "default backend should be GitHub");
        assert_eq!(cfg.columns.len(), 5, "default should have 5 columns");

        // Verify config file was NOT created by load()
        let path = config_path();
        assert!(!path.exists(), "config file should NOT be created by load()");

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

        let path = cache_file_path("test/repo");
        std::fs::write(&path, r#"{"last_sync": "2024-01-01"}"#).unwrap();

        assert!(read_cache("test/repo").is_none(), "should return None when issues key missing");

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
    fn test_cache_file_path_format() {
        let path = cache_file_path("owner/name");
        let filename = path.file_name().unwrap().to_str().unwrap().to_string();
        assert_eq!(filename, "issues-owner-name.json");
    }

    #[test]
    fn test_cache_file_path_multi_slash() {
        let path = cache_file_path("a/b/c");
        let filename = path.file_name().unwrap().to_str().unwrap().to_string();
        assert_eq!(filename, "issues-a-b-c.json");
    }

    #[test]
    fn test_read_cache_meta_roundtrip() {
        let _lock = CACHE_LOCK.lock().unwrap();

        let tmp = std::env::temp_dir().join(format!("git-kanban-test-meta-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CACHE_HOME", &tmp);

        let issues = make_test_issues();
        write_cache(&issues, "2024-06-15T10:30:00Z", "meta/repo");

        let result = read_cache_meta("meta/repo");
        assert!(result.is_some());
        let (loaded_issues, last_sync) = result.unwrap();
        assert_eq!(loaded_issues.len(), 2);
        assert_eq!(last_sync, "2024-06-15T10:30:00Z");
        assert_eq!(loaded_issues[0].number, 1);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_cache_meta_no_file() {
        let _lock = CACHE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-nometa-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CACHE_HOME", &tmp);

        assert!(read_cache_meta("meta/repo").is_none());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_write_cache_creates_dir() {
        let _lock = CACHE_LOCK.lock().unwrap();

        // Use a deeply nested temp dir that doesn't exist yet
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-mkdir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        // Do NOT create git-kanban subdir — write_cache should create it
        std::env::set_var("XDG_CACHE_HOME", &tmp);

        let issues = make_test_issues();
        let result = write_cache(&issues, "now", "create/dir");
        assert!(result, "write_cache should create cache dir if missing");

        let path = cache_file_path("create/dir");
        assert!(path.exists(), "cache file should exist after write_cache");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_with_extra_config_fields() {
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-extra-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let config_dir = tmp.join("git-kanban");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.json"),
            r#"{"repo": "o/r", "note": "some note", "unknown_field": 42}"#,
        ).unwrap();

        let cfg = load();
        assert_eq!(cfg.repo, "o/r");
        assert_eq!(cfg.backend, Backend::GitHub);
        assert_eq!(cfg.columns.len(), 5);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_config_has_both_repo_and_backend() {
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-rb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let config_dir = tmp.join("git-kanban");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.json"),
            r#"{"repo": "group/project", "backend": "gitlab"}"#,
        ).unwrap();

        let cfg = load();
        assert_eq!(cfg.repo, "group/project");
        assert_eq!(cfg.backend, Backend::GitLab);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_config_columns_partial_override() {
        // Only override "done" column labels, others stay default
        let _lock = CACHE_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("git-kanban-test-part-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);

        let config_dir = tmp.join("git-kanban");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.json"),
            r#"{"columns": {"done": ["completed", "verified"]}}"#,
        ).unwrap();

        let cfg = load();
        assert_eq!(cfg.columns[0].labels, vec!["todo", "status:todo"], "todo should stay default");
        assert_eq!(cfg.columns[3].id, "done");
        assert_eq!(cfg.columns[3].labels, vec!["completed", "verified"], "done should be overridden");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
