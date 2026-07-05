use serde::{Deserialize, Serialize};
use std::fmt;

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
    pub state: String,
    pub labels: Vec<GhLabel>,
    pub assignees: Vec<GhAssignee>,
    pub created_at: String,
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
    pub labels: Vec<String>,   // issues with any of these labels appear here
    pub show_closed: bool,     // if true, show state=closed issues
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
        self.labels.iter().any(|l| issue.labels.contains(l))
    }
}

#[derive(Debug, Clone)]
pub struct Cache {
    pub last_sync: String,
    pub issues: Vec<Issue>,
}

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
