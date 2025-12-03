use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Config/Data 差分の重大度。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSeverity {
    Info,
    Warning,
    Error,
}

impl Default for ChangeSeverity {
    fn default() -> Self {
        ChangeSeverity::Warning
    }
}

/// 差分操作の種類。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeOperation {
    Added,
    Removed,
    Updated,
}

/// 個別の差分エントリ。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeEntry {
    pub op: ChangeOperation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<Value>,
    #[serde(default)]
    pub severity: ChangeSeverity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ChangeEntry {
    pub fn added(path: Vec<String>, current: Value) -> Self {
        Self {
            op: ChangeOperation::Added,
            path,
            previous: None,
            current: Some(current),
            severity: ChangeSeverity::Warning,
            message: None,
        }
    }

    pub fn removed(path: Vec<String>, previous: Value) -> Self {
        Self {
            op: ChangeOperation::Removed,
            path,
            previous: Some(previous),
            current: None,
            severity: ChangeSeverity::Warning,
            message: None,
        }
    }

    pub fn updated(path: Vec<String>, previous: Value, current: Value) -> Self {
        Self {
            op: ChangeOperation::Updated,
            path,
            previous: Some(previous),
            current: Some(current),
            severity: ChangeSeverity::Warning,
            message: None,
        }
    }

    pub fn with_severity(mut self, severity: ChangeSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// 差分全体とサマリ。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeSet {
    pub entries: Vec<ChangeEntry>,
    pub summary: ChangeSummary,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            summary: ChangeSummary::default(),
        }
    }

    pub fn from_entries(entries: Vec<ChangeEntry>) -> Self {
        let mut summary = ChangeSummary::default();
        for entry in &entries {
            summary.register(entry);
        }
        Self { entries, summary }
    }

    pub fn push(&mut self, entry: ChangeEntry) {
        self.summary.register(&entry);
        self.entries.push(entry);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 差分サマリ。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ChangeSummary {
    pub added: usize,
    pub removed: usize,
    pub updated: usize,
}

impl ChangeSummary {
    fn register(&mut self, entry: &ChangeEntry) {
        match entry.op {
            ChangeOperation::Added => self.added = self.added.saturating_add(1),
            ChangeOperation::Removed => self.removed = self.removed.saturating_add(1),
            ChangeOperation::Updated => self.updated = self.updated.saturating_add(1),
        }
    }

    pub fn total(&self) -> usize {
        self.added + self.removed + self.updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn summary_tracks_counts() {
        let mut change_set = ChangeSet::new();
        change_set.push(ChangeEntry::added(vec!["project".into()], json!("app")));
        change_set.push(ChangeEntry::removed(vec!["dsl".into()], json!("old")));
        change_set.push(ChangeEntry::updated(
            vec!["build".into(), "optimize".into()],
            json!("debug"),
            json!("release"),
        ));
        assert_eq!(change_set.summary.added, 1);
        assert_eq!(change_set.summary.removed, 1);
        assert_eq!(change_set.summary.updated, 1);
        assert_eq!(change_set.summary.total(), 3);
    }

    #[test]
    fn severity_and_message_are_optional() {
        let entry = ChangeEntry::added(vec!["schema".into()], json!(1))
            .with_severity(ChangeSeverity::Error)
            .with_message("breaking change");
        assert_eq!(entry.severity, ChangeSeverity::Error);
        assert_eq!(entry.message.as_deref(), Some("breaking change"));
    }
}
