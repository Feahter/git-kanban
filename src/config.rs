use std::path::PathBuf;
use crate::types::Config;

const CONFIG_DIR: &str = "gh-kanban";
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
            "note": "Set your repo in format 'owner/name'. Remove labels you don't use, add yours."
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
