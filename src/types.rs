use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Backend {
    #[serde(rename = "github")]
    GitHub,
    #[serde(rename = "gitlab")]
    GitLab,
}

impl Default for Backend {
    fn default() -> Self {
        Backend::GitHub
    }
}

impl Backend {
    pub fn cmd(&self) -> &'static str {
        match self {
            Backend::GitHub => "gh",
            Backend::GitLab => "glab",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
}

impl Priority {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhIssue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub labels: Vec<GhLabel>,
    pub assignees: Vec<GhAssignee>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhLabel {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhAssignee {
    pub login: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub state: IssueState,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub priority: Option<Priority>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueState {
    Open,
    Closed,
}

#[derive(Debug, Clone)]
pub struct Column {
    pub id: String,
    pub title: String,
    pub labels: Vec<String>,
    pub show_closed: bool,
    pub issues: Vec<Issue>,
}

impl Column {
    pub fn matches(&self, issue: &Issue) -> bool {
        if issue.state == IssueState::Closed {
            return self.show_closed;
        }
        if self.labels.is_empty() {
            return false;
        }
        self.labels.iter().any(|l| issue.labels.iter().any(|il| il.eq_ignore_ascii_case(l)))
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub repo: String,
    pub repos: Vec<String>,
    pub backend: Backend,
    pub columns: Vec<Column>,
}

impl Config {
    /// Return the effective repo list: `repos` takes priority, fallback to single `repo`.
    pub fn repo_list(&self) -> Vec<String> {
        if !self.repos.is_empty() {
            self.repos.clone()
        } else if !self.repo.is_empty() {
            vec![self.repo.clone()]
        } else {
            vec![]
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            repo: String::new(),
            repos: vec![],
            backend: Backend::default(),
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

// ── Unit tests ──
#[cfg(test)]
mod tests {
    use super::*;

    // ── Priority tests ──

    #[test]
    fn test_priority_from_labels_p0() {
        let labels = vec!["p0".to_string()];
        assert_eq!(Priority::from_labels(&labels), Some(Priority::P0));
    }

    #[test]
    fn test_priority_from_labels_p1() {
        let labels = vec!["p1".to_string()];
        assert_eq!(Priority::from_labels(&labels), Some(Priority::P1));
    }

    #[test]
    fn test_priority_from_labels_p2() {
        let labels = vec!["p2".to_string()];
        assert_eq!(Priority::from_labels(&labels), Some(Priority::P2));
    }

    #[test]
    fn test_priority_from_labels_p3() {
        let labels = vec!["p3".to_string()];
        assert_eq!(Priority::from_labels(&labels), Some(Priority::P3));
    }

    #[test]
    fn test_priority_from_labels_priority_colon() {
        let labels = vec!["priority:1".to_string()];
        assert_eq!(Priority::from_labels(&labels), Some(Priority::P1));
    }

    #[test]
    fn test_priority_from_labels_priority_dash() {
        let labels = vec!["priority-p2".to_string()];
        assert_eq!(Priority::from_labels(&labels), Some(Priority::P2));
    }

    #[test]
    fn test_priority_from_labels_case_insensitive() {
        let labels = vec!["P0".to_string(), "PRIORITY:3".to_string()];
        assert_eq!(Priority::from_labels(&labels), Some(Priority::P0));
    }

    #[test]
    fn test_priority_from_labels_none() {
        let labels = vec!["bug".to_string(), "feature".to_string()];
        assert_eq!(Priority::from_labels(&labels), None);
    }

    #[test]
    fn test_priority_from_labels_empty() {
        let labels: Vec<String> = vec![];
        assert_eq!(Priority::from_labels(&labels), None);
    }

    #[test]
    fn test_priority_from_labels_first_match_wins() {
        let labels = vec!["p2".to_string(), "p0".to_string()];
        assert_eq!(Priority::from_labels(&labels), Some(Priority::P2));
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(format!("{}", Priority::P0), "P0");
        assert_eq!(format!("{}", Priority::P1), "P1");
        assert_eq!(format!("{}", Priority::P2), "P2");
        assert_eq!(format!("{}", Priority::P3), "P3");
    }

    #[test]
    fn test_priority_serde_roundtrip() {
        for p in &[Priority::P0, Priority::P1, Priority::P2, Priority::P3] {
            let json = serde_json::to_string(p).unwrap();
            let deserialized: Priority = serde_json::from_str(&json).unwrap();
            assert_eq!(*p, deserialized);
        }
    }

    // ── IssueState serde ──

    #[test]
    fn test_issue_state_serde_roundtrip() {
        for state in &[IssueState::Open, IssueState::Closed] {
            let json = serde_json::to_string(state).unwrap();
            let deserialized: IssueState = serde_json::from_str(&json).unwrap();
            assert_eq!(*state, deserialized);
        }
    }

    // ── Backend tests ──

    #[test]
    fn test_backend_default() {
        assert_eq!(Backend::default(), Backend::GitHub);
    }

    #[test]
    fn test_backend_cmd() {
        assert_eq!(Backend::GitHub.cmd(), "gh");
        assert_eq!(Backend::GitLab.cmd(), "glab");
    }

    #[test]
    fn test_backend_serde_roundtrip() {
        for b in &[Backend::GitHub, Backend::GitLab] {
            let json = serde_json::to_string(b).unwrap();
            let deserialized: Backend = serde_json::from_str(&json).unwrap();
            assert_eq!(*b, deserialized);
        }
    }

    // ── Column::matches() tests ──

    fn make_open_issue(labels: Vec<String>) -> Issue {
        Issue {
            number: 1,
            title: "test".into(),
            body: String::new(),
            state: IssueState::Open,
            labels,
            assignees: vec![],
            priority: None,
            created_at: "now".into(),
            updated_at: "now".into(),
        }
    }

    fn make_closed_issue(labels: Vec<String>) -> Issue {
        Issue {
            number: 2,
            title: "closed".into(),
            body: String::new(),
            state: IssueState::Closed,
            labels,
            assignees: vec![],
            priority: None,
            created_at: "now".into(),
            updated_at: "now".into(),
        }
    }

    #[test]
    fn test_column_matches_label_exact() {
        let col = Column {
            id: "todo".into(),
            title: "Todo".into(),
            labels: vec!["todo".into()],
            show_closed: false,
            issues: vec![],
        };
        assert!(col.matches(&make_open_issue(vec!["todo".into()])));
    }

    #[test]
    fn test_column_matches_any_of_labels() {
        let col = Column {
            id: "doing".into(),
            title: "Doing".into(),
            labels: vec!["doing".into(), "in-progress".into()],
            show_closed: false,
            issues: vec![],
        };
        assert!(col.matches(&make_open_issue(vec!["in-progress".into()])));
    }

    #[test]
    fn test_column_matches_no_match() {
        let col = Column {
            id: "todo".into(),
            title: "Todo".into(),
            labels: vec!["todo".into()],
            show_closed: false,
            issues: vec![],
        };
        assert!(!col.matches(&make_open_issue(vec!["bug".into()])));
    }

    #[test]
    fn test_column_matches_empty_labels_open_issue() {
        let col = Column {
            id: "custom".into(),
            title: "Custom".into(),
            labels: vec![],
            show_closed: false,
            issues: vec![],
        };
        assert!(!col.matches(&make_open_issue(vec!["anything".into()])));
    }

    #[test]
    fn test_column_matches_closed_with_show_closed() {
        let col = Column {
            id: "done".into(),
            title: "Done".into(),
            labels: vec!["done".into()],
            show_closed: true,
            issues: vec![],
        };
        assert!(col.matches(&make_closed_issue(vec!["done".into()])));
    }

    #[test]
    fn test_column_matches_closed_without_show_closed() {
        let col = Column {
            id: "todo".into(),
            title: "Todo".into(),
            labels: vec!["todo".into()],
            show_closed: false,
            issues: vec![],
        };
        assert!(!col.matches(&make_closed_issue(vec!["todo".into()])));
    }

    #[test]
    fn test_column_matches_closed_show_closed_empty_labels() {
        let col = Column {
            id: "closed".into(),
            title: "Closed".into(),
            labels: vec![],
            show_closed: true,
            issues: vec![],
        };
        assert!(col.matches(&make_closed_issue(vec![])));
    }

    // ── Issue construction ──

    #[test]
    fn test_issue_fields() {
        let issue = Issue {
            number: 42,
            title: "Test Issue".into(),
            body: "Body text".into(),
            state: IssueState::Open,
            labels: vec!["bug".into()],
            assignees: vec!["user1".into()],
            priority: Some(Priority::P1),
            created_at: "2024-01-01".into(),
            updated_at: "2024-01-02".into(),
        };
        assert_eq!(issue.number, 42);
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.body, "Body text");
        assert_eq!(issue.state, IssueState::Open);
        assert_eq!(issue.labels, vec!["bug"]);
        assert_eq!(issue.assignees, vec!["user1"]);
        assert_eq!(issue.priority, Some(Priority::P1));
    }

    #[test]
    fn test_column_matches_closed_no_show_closed_no_labels() {
        let col = Column {
            id: "custom".into(),
            title: "Custom".into(),
            labels: vec![],
            show_closed: false,
            issues: vec![],
        };
        assert!(!col.matches(&make_closed_issue(vec![])));
    }

    #[test]
    fn test_column_matches_closed_no_show_closed_with_labels() {
        let col = Column {
            id: "todo".into(),
            title: "Todo".into(),
            labels: vec!["todo".into()],
            show_closed: false,
            issues: vec![],
        };
        // Closed check happens first — even with matching labels, returns false
        assert!(!col.matches(&make_closed_issue(vec!["todo".into()])));
    }

    #[test]
    fn test_column_matches_open_multi_label_issue() {
        let col = Column {
            id: "doing".into(),
            title: "Doing".into(),
            labels: vec!["doing".into(), "in-progress".into()],
            show_closed: false,
            issues: vec![],
        };
        assert!(col.matches(&make_open_issue(vec![
            "bug".into(),
            "in-progress".into(),
            "urgent".into(),
        ])));
    }

    #[test]
    fn test_column_matches_case_sensitive() {
        let col = Column {
            id: "todo".into(),
            title: "Todo".into(),
            labels: vec!["TODO".into()],
            show_closed: false,
            issues: vec![],
        };
        assert!(col.matches(&make_open_issue(vec!["todo".into()])));
    }

    #[test]
    fn test_priority_from_labels_bogus_priority() {
        let labels = vec!["priority:10".to_string()];
        assert_eq!(Priority::from_labels(&labels), None);
    }

    #[test]
    fn test_issue_serde_full_roundtrip() {
        let issue = Issue {
            number: 99,
            title: "Roundtrip".into(),
            body: "Body text".into(),
            state: IssueState::Closed,
            labels: vec!["bug".into(), "urgent".into()],
            assignees: vec!["alice".into(), "bob".into()],
            priority: Some(Priority::P0),
            created_at: "2024-06-01T00:00:00Z".into(),
            updated_at: "2024-06-02T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&issue).unwrap();
        let deserialized: Issue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.number, 99);
        assert_eq!(deserialized.title, "Roundtrip");
        assert_eq!(deserialized.body, "Body text");
        assert_eq!(deserialized.state, IssueState::Closed);
        assert_eq!(deserialized.labels, vec!["bug", "urgent"]);
        assert_eq!(deserialized.assignees, vec!["alice", "bob"]);
        assert_eq!(deserialized.priority, Some(Priority::P0));
    }

    #[test]
    fn test_issue_serde_body_default() {
        let json = r#"{"number":5,"title":"No Body","state":"Open","labels":[],"assignees":[],"created_at":"now","updated_at":"now"}"#;
        let issue: Issue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.number, 5);
        assert_eq!(issue.body, ""); // #[serde(default)] kicks in
    }

    // ── Config::default() tests ──

    #[test]
    fn test_config_default_repo_empty() {
        let cfg = Config::default();
        assert!(cfg.repo.is_empty(), "default repo should be empty");
    }

    #[test]
    fn test_config_default_backend_github() {
        let cfg = Config::default();
        assert_eq!(cfg.backend, Backend::GitHub);
    }

    #[test]
    fn test_config_default_has_five_columns() {
        let cfg = Config::default();
        assert_eq!(cfg.columns.len(), 5);
    }

    #[test]
    fn test_config_default_column_ids() {
        let cfg = Config::default();
        let ids: Vec<&str> = cfg.columns.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["todo", "doing", "review", "done", "closed"]);
    }

    #[test]
    fn test_config_default_closed_column_show_closed() {
        let cfg = Config::default();
        assert!(cfg.columns[4].show_closed);
        assert!(cfg.columns[4].labels.is_empty());
    }

    #[test]
    fn test_config_default_column_titles() {
        let cfg = Config::default();
        assert_eq!(cfg.columns[0].title, "📋 Todo");
        assert_eq!(cfg.columns[1].title, "🔧 Doing");
        assert_eq!(cfg.columns[2].title, "👀 Review");
        assert_eq!(cfg.columns[3].title, "✅ Done");
        assert_eq!(cfg.columns[4].title, "❌ Closed");
    }

    #[test]
    fn test_config_default_column_lists() {
        let cfg = Config::default();
        assert_eq!(cfg.columns[0].labels, vec!["todo", "status:todo"]);
        assert_eq!(cfg.columns[1].labels, vec!["doing", "status:doing", "in-progress"]);
        assert_eq!(cfg.columns[2].labels, vec!["review", "status:review"]);
        assert_eq!(cfg.columns[3].labels, vec!["done", "status:done"]);
        assert!(cfg.columns[4].labels.is_empty());
    }

    // ── More Column::matches() edge cases ──

    #[test]
    fn test_column_matches_issue_labels_uppercase_column_labels_lowercase() {
        // Column labels are lowercase, issue has UPPERCASE labels
        let col = Column {
            id: "todo".into(), title: "Todo".into(),
            labels: vec!["todo".into()], show_closed: false, issues: vec![],
        };
        assert!(col.matches(&make_open_issue(vec!["TODO".into()])));
    }

    #[test]
    fn test_column_matches_issue_has_multiple_labels_matches_last() {
        let col = Column {
            id: "doing".into(), title: "Doing".into(),
            labels: vec!["doing".into(), "in-progress".into(), "wip".into()],
            show_closed: false, issues: vec![],
        };
        assert!(col.matches(&make_open_issue(vec!["bug".into(), "wip".into()])));
    }

    #[test]
    fn test_column_matches_issue_no_labels_open_column_has_labels() {
        let col = Column {
            id: "todo".into(), title: "Todo".into(),
            labels: vec!["todo".into()], show_closed: false, issues: vec![],
        };
        assert!(!col.matches(&make_open_issue(vec![])));
    }

    #[test]
    fn test_column_matches_issue_no_labels_open_column_empty_labels() {
        // Open issue with no labels, column with empty labels → false
        let col = Column {
            id: "custom".into(), title: "Custom".into(),
            labels: vec![], show_closed: false, issues: vec![],
        };
        assert!(!col.matches(&make_open_issue(vec![])));
    }

    // ── Backend serde string tests ──

    #[test]
    fn test_backend_serde_github_string() {
        let json = serde_json::to_string(&Backend::GitHub).unwrap();
        assert_eq!(json, "\"github\"");
    }

    #[test]
    fn test_backend_serde_gitlab_string() {
        let json = serde_json::to_string(&Backend::GitLab).unwrap();
        assert_eq!(json, "\"gitlab\"");
    }

    #[test]
    fn test_backend_deserialize_github() {
        let b: Backend = serde_json::from_str("\"github\"").unwrap();
        assert_eq!(b, Backend::GitHub);
    }

    #[test]
    fn test_backend_deserialize_gitlab() {
        let b: Backend = serde_json::from_str("\"gitlab\"").unwrap();
        assert_eq!(b, Backend::GitLab);
    }

    #[test]
    fn test_backend_deserialize_invalid() {
        let result: Result<Backend, _> = serde_json::from_str("\"bitbucket\"");
        assert!(result.is_err(), "invalid backend should fail to deserialize");
    }
}
