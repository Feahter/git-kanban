use serde::{Deserialize, Serialize};
use std::fmt;

/// Priority level extracted from issue labels.
/// Supports `P0`–`P3` and `priority:0`–`priority:3` label conventions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
}

impl Priority {
    /// Parse a priority from a slice of label strings.
    /// Returns `None` if no recognised priority label is found.
    pub fn from_labels(labels: &[String]) -> Option<Self> {
        for l in labels {
            let lower = l.to_lowercase();
            if lower == "p0" || lower == "priority:0" || lower == "priority-p0" {
                return Some(Priority::P0);
            }
            if lower == "p1" || lower == "priority:1" || lower == "priority-p1" {
                return Some(Priority::P1);
            }
            if lower == "p2" || lower == "priority:2" || lower == "priority-p2" {
                return Some(Priority::P2);
            }
            if lower == "p3" || lower == "priority:3" || lower == "priority-p3" {
                return Some(Priority::P3);
            }
        }
        None
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::P0 => write!(f, "P0"),
            Priority::P1 => write!(f, "P1"),
            Priority::P2 => write!(f, "P2"),
            Priority::P3 => write!(f, "P3"),
        }
    }
}

/// Raw issue response from `gh issue list --json`, mirroring the API shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GhIssue {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub labels: Vec<GhLabel>,
    pub assignees: Vec<GhAssignee>,
    pub created_at: String,
    pub updated_at: String,
}

/// A label as returned by the GitHub API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhLabel {
    pub name: String,
}

/// An assignee as returned by the GitHub API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhAssignee {
    pub login: String,
}

/// A processed GitHub issue with resolved labels and priority.
///
/// Fields are flattened for direct JSON serialization — agents use this
/// via `--json` output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub state: IssueState,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub priority: Option<Priority>,
    pub created_at: String,
    pub updated_at: String,
}

/// Whether a GitHub issue is open or closed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueState {
    Open,
    Closed,
}

/// A kanban column that groups issues matching specific labels.
#[derive(Debug, Clone)]
pub struct Column {
    pub id: String,
    pub title: String,
    pub labels: Vec<String>,   // issues with any of these labels appear here
    pub show_closed: bool,     // if true, show state=closed issues
    pub issues: Vec<Issue>,
}

impl Column {
    /// Returns `true` if `issue` belongs in this column.
    ///
    /// * Closed issues belong only to columns with `show_closed: true`.
    /// * Open issues must have at least one label matching the column's label list.
    pub fn matches(&self, issue: &Issue) -> bool {
        if issue.state == IssueState::Closed {
            return self.show_closed;
        }
        if self.labels.is_empty() {
            return false;
        }
        self.labels.iter().any(|l| issue.labels.contains(l))
    }
}

#[derive(Debug, Clone)]
pub struct Cache {
    pub last_sync: String,
    pub issues: Vec<Issue>,
}

/// In-memory config with resolved repo and column definitions.
#[derive(Debug, Clone)]
pub struct Config {
    pub repo: String,
    pub columns: Vec<Column>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            repo: String::new(),
            columns: vec![
                Column {
                    id: "todo".into(),
                    title: "📋 Todo".into(),
                    labels: vec!["todo".into(), "status:todo".into()],
                    show_closed: false,
                    issues: vec![],
                },
                Column {
                    id: "doing".into(),
                    title: "🔧 Doing".into(),
                    labels: vec!["doing".into(), "status:doing".into(), "in-progress".into()],
                    show_closed: false,
                    issues: vec![],
                },
                Column {
                    id: "review".into(),
                    title: "👀 Review".into(),
                    labels: vec!["review".into(), "status:review".into()],
                    show_closed: false,
                    issues: vec![],
                },
                Column {
                    id: "done".into(),
                    title: "✅ Done".into(),
                    labels: vec!["done".into(), "status:done".into()],
                    show_closed: false,
                    issues: vec![],
                },
                Column {
                    id: "closed".into(),
                    title: "❌ Closed".into(),
                    labels: vec![],
                    show_closed: true,
                    issues: vec![],
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Priority ──

    #[test]
    fn test_priority_p0_all_formats() {
        let labels = |s: &str| vec![s.to_string()];
        assert_eq!(Priority::from_labels(&labels("p0")), Some(Priority::P0));
        assert_eq!(Priority::from_labels(&labels("P0")), Some(Priority::P0));
        assert_eq!(Priority::from_labels(&labels("priority:0")), Some(Priority::P0));
        assert_eq!(Priority::from_labels(&labels("priority-p0")), Some(Priority::P0));
        assert_eq!(Priority::from_labels(&labels("PRIORITY:0")), Some(Priority::P0));
    }

    #[test]
    fn test_priority_all_levels() {
        for (label, expected) in [
            ("p0", Priority::P0),
            ("p1", Priority::P1),
            ("p2", Priority::P2),
            ("p3", Priority::P3),
        ] {
            let labels = vec![label.to_string(), "other".to_string()];
            assert_eq!(Priority::from_labels(&labels), Some(expected), "label={label}");
        }
    }

    #[test]
    fn test_priority_returns_first_match() {
        let labels = vec!["bug".into(), "p2".into(), "p1".into()];
        assert_eq!(Priority::from_labels(&labels), Some(Priority::P2));
    }

    #[test]
    fn test_priority_no_match() {
        assert_eq!(Priority::from_labels(&[]), None);
        assert_eq!(Priority::from_labels(&["bug".into(), "feature".into()]), None);
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(format!("{}", Priority::P0), "P0");
        assert_eq!(format!("{}", Priority::P1), "P1");
        assert_eq!(format!("{}", Priority::P2), "P2");
        assert_eq!(format!("{}", Priority::P3), "P3");
    }

    // ── Column::matches ──

    fn open_issue(labels: &[&str]) -> Issue {
        Issue {
            number: 1,
            title: "test".into(),
            state: IssueState::Open,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            assignees: vec![],
            priority: None,
            created_at: "".into(),
            updated_at: "".into(),
        }
    }

    fn closed_issue() -> Issue {
        Issue {
            number: 2,
            title: "closed".into(),
            state: IssueState::Closed,
            labels: vec![],
            assignees: vec![],
            priority: None,
            created_at: "".into(),
            updated_at: "".into(),
        }
    }

    #[test]
    fn test_column_matches_open_with_matching_label() {
        let col = Column {
            id: "doing".into(),
            title: "Doing".into(),
            labels: vec!["doing".into(), "in-progress".into()],
            show_closed: false,
            issues: vec![],
        };
        assert!(col.matches(&open_issue(&["doing"])));
        assert!(col.matches(&open_issue(&["bug", "doing"])));
        assert!(!col.matches(&open_issue(&["todo"])));
        assert!(!col.matches(&open_issue(&[])));
    }

    #[test]
    fn test_column_matches_closed_only_show_closed() {
        let col_with_closed = Column {
            id: "closed".into(),
            title: "Closed".into(),
            labels: vec![],
            show_closed: true,
            issues: vec![],
        };
        let col_without_closed = Column {
            id: "todo".into(),
            title: "Todo".into(),
            labels: vec!["todo".into()],
            show_closed: false,
            issues: vec![],
        };
        assert!(col_with_closed.matches(&closed_issue()));
        assert!(!col_without_closed.matches(&closed_issue()));
    }

    #[test]
    fn test_column_empty_labels_never_match_open() {
        let col = Column {
            id: "empty".into(),
            title: "Empty".into(),
            labels: vec![],
            show_closed: false,
            issues: vec![],
        };
        assert!(!col.matches(&open_issue(&["anything"])));
        assert!(!col.matches(&closed_issue())); // show_closed=false
    }

    // ── Config Default ──

    #[test]
    fn test_config_default_has_five_columns() {
        let cfg = Config::default();
        assert_eq!(cfg.columns.len(), 5);
        assert_eq!(cfg.columns[0].id, "todo");
        assert_eq!(cfg.columns[1].id, "doing");
        assert_eq!(cfg.columns[2].id, "review");
        assert_eq!(cfg.columns[3].id, "done");
        assert_eq!(cfg.columns[4].id, "closed");
    }

    #[test]
    fn test_config_default_closed_column() {
        let cfg = Config::default();
        let closed = &cfg.columns[4];
        assert!(closed.show_closed);
        assert!(closed.labels.is_empty());
    }

    // ── Issue JSON serialization ──

    #[test]
    fn test_issue_serialization_agent_format() {
        let issue = Issue {
            number: 42,
            title: "Test issue".into(),
            state: IssueState::Open,
            labels: vec!["bug".into(), "p0".into()],
            assignees: vec!["user1".into()],
            priority: Some(Priority::P0),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-06-01T12:00:00Z".into(),
        };
        let json = serde_json::to_value(&issue).unwrap();
        let obj = json.as_object().unwrap();
        assert_eq!(obj["number"], 42);
        assert_eq!(obj["title"], "Test issue");
        assert_eq!(obj["state"], "Open");
        assert_eq!(obj["priority"], "P0");
        assert!(obj.get("labels").unwrap().is_array());
        assert!(obj.get("assignees").unwrap().is_array());
        assert_eq!(obj["labels"].as_array().unwrap().len(), 2);
    }
}
