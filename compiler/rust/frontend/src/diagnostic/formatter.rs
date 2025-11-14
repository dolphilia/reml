use crate::diagnostic::{AuditEnvelope, FrontendDiagnostic};
use serde_json::{json, Map, Value};
use std::env;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const AUDIT_POLICY_VERSION: &str = "rust.poc.audit.v1";

static AUDIT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
/// CLI から渡されるコンテキスト情報。診断ごとに `change_set` を組み立てるために用いる。
pub struct FormatterContext<'a> {
    pub program_name: &'a str,
    pub raw_args: &'a [String],
    pub input_path: &'a Path,
}

impl<'a> FormatterContext<'a> {
    fn change_set(&self) -> Value {
        json!({
            "policy": AUDIT_POLICY_VERSION,
            "origin": "cli",
            "source": {
                "command": self.program_name,
                "args": self.raw_args,
                "workspace": default_workspace_root(),
            },
            "items": [
                {
                    "kind": "cli-command",
                    "command": self.program_name,
                    "args": self.raw_args,
                },
                {
                    "kind": "input",
                    "path": self.input_path.display().to_string(),
                    "target": "rust-poc",
                }
            ],
        })
    }
}

/// 現在時刻の文字列表現（`YYYY-MM-DDTHH:MM:SSZ`）を返す。
pub fn current_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    let seconds = duration.as_secs() as i64;
    let (year, month, day, hour, minute, second) = unix_seconds_to_components(seconds);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

pub fn finalize_audit_metadata(
    metadata: &mut Map<String, Value>,
    diag: &mut FrontendDiagnostic,
    timestamp: &str,
    context: &FormatterContext<'_>,
    primary_capability: Option<&str>,
) -> AuditEnvelope {
    let envelope = complete_audit_metadata(metadata, timestamp, context, primary_capability);
    diag.audit_metadata = metadata.clone();
    diag.audit = envelope.clone();
    envelope
}

pub fn complete_audit_metadata(
    metadata: &mut Map<String, Value>,
    timestamp: &str,
    context: &FormatterContext<'_>,
    primary_capability: Option<&str>,
) -> AuditEnvelope {
    let normalized_timestamp = normalize_timestamp(timestamp);
    metadata.insert(
        "audit.timestamp".to_string(),
        json!(normalized_timestamp.clone()),
    );
    let change_set = context.change_set();
    metadata.insert("cli.change_set".to_string(), change_set.clone());
    let audit_id = ensure_audit_id(metadata, &normalized_timestamp, "auto");
    AuditEnvelope::from_parts(
        metadata.clone(),
        Some(audit_id),
        Some(change_set),
        primary_capability.map(|cap| cap.to_string()),
    )
}

fn normalize_timestamp(value: &str) -> String {
    if !value.trim().is_empty() {
        value.to_string()
    } else {
        current_timestamp()
    }
}

fn ensure_audit_id(metadata: &mut Map<String, Value>, timestamp: &str, prefix: &str) -> String {
    if let Some(Value::String(existing)) = metadata.get("cli.audit_id") {
        if !existing.trim().is_empty() {
            return existing.clone();
        }
    }
    let channel = channel_from_prefix(prefix, metadata);
    let existing_commit = metadata_string(metadata, "audit.source.commit");
    let commit = existing_commit.or_else(commit_hint);
    let workspace =
        metadata_string(metadata, "audit.source.workspace").unwrap_or_else(default_workspace_root);
    let existing_build_id = metadata_string(metadata, "audit.build_id");
    let build_id = compute_build_id(timestamp, existing_build_id.as_deref(), commit.as_deref());
    let sequence = next_audit_sequence();
    apply_audit_policy_metadata(
        metadata,
        &channel,
        &build_id,
        sequence,
        commit.as_deref(),
        Some(workspace.as_str()),
    );
    let audit_id = format!("{channel}/{build_id}#{sequence}");
    metadata.insert("cli.audit_id".to_string(), json!(audit_id.clone()));
    audit_id
}

fn channel_from_prefix(prefix: &str, metadata: &Map<String, Value>) -> String {
    let trimmed = prefix.trim();
    let normalized = match trimmed {
        "" | "auto" => None,
        "legacy" => Some("legacy-import".to_string()),
        value => Some(value.to_string()),
    };
    normalized.unwrap_or_else(|| {
        metadata_string(metadata, "audit.channel").unwrap_or_else(|| "cli".to_string())
    })
}

fn compute_build_id(timestamp: &str, existing: Option<&str>, commit: Option<&str>) -> String {
    if let Some(value) = existing {
        if !value.trim().is_empty() {
            return value.to_string();
        }
    }
    let base = compact_timestamp(timestamp);
    match commit {
        Some(value) if !value.trim().is_empty() => format!("{base}-{value}"),
        _ => base,
    }
}

fn compact_timestamp(timestamp: &str) -> String {
    let buffer = timestamp
        .chars()
        .filter(|ch| *ch != '-' && *ch != ':')
        .collect::<String>();
    if buffer.trim().is_empty() {
        "00000000000000".to_string()
    } else {
        buffer
    }
}

fn next_audit_sequence() -> u64 {
    AUDIT_SEQUENCE.fetch_add(1, Ordering::SeqCst)
}

fn apply_audit_policy_metadata(
    metadata: &mut Map<String, Value>,
    channel: &str,
    build_id: &str,
    sequence: u64,
    commit: Option<&str>,
    workspace: Option<&str>,
) {
    metadata.insert(
        "audit.policy.version".to_string(),
        json!(AUDIT_POLICY_VERSION),
    );
    metadata.insert("audit.channel".to_string(), json!(channel));
    metadata.insert("audit.build_id".to_string(), json!(build_id));
    metadata.insert("audit.sequence".to_string(), json!(sequence));
    if let Some(workspace) = workspace {
        if !workspace.trim().is_empty() {
            metadata.insert("audit.source.workspace".to_string(), json!(workspace));
        }
    }
    if let Some(commit) = commit {
        if !commit.trim().is_empty() {
            metadata.insert("audit.source.commit".to_string(), json!(commit));
        }
    }
}

fn default_workspace_root() -> String {
    match env::var("REMLC_WORKSPACE_ROOT") {
        Ok(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => ".".to_string(),
    }
}

fn commit_hint() -> Option<String> {
    env::var("REMLC_GIT_COMMIT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("GITHUB_SHA")
                .ok()
                .map(|value| {
                    let trimmed = value.trim();
                    let len = trimmed.len().min(7);
                    trimmed[..len].to_string()
                })
                .filter(|value| !value.is_empty())
        })
}

fn metadata_string(metadata: &Map<String, Value>, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn unix_seconds_to_components(seconds: i64) -> (i32, u32, u32, u32, u32, u32) {
    const SECONDS_PER_DAY: i64 = 86_400;
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let mut rem = seconds.rem_euclid(SECONDS_PER_DAY);
    if rem < 0 {
        rem += SECONDS_PER_DAY;
    }
    let hour = (rem / 3_600) as u32;
    rem %= 3_600;
    let minute = (rem / 60) as u32;
    let second = (rem % 60) as u32;
    let (year, month, day) = unix_days_to_date(days);
    (year, month, day, hour, minute, second)
}

fn unix_days_to_date(days: i64) -> (i32, u32, u32) {
    // Gregorian calendar conversion (Julian day number offset).
    let a = days + 68569;
    let b = 4 * a / 146097;
    let c = a - (146097 * b + 3) / 4;
    let d = 4000 * (c + 1) / 1461001;
    let e = c - 1461 * d / 4 + 31;
    let f = 80 * e / 2447;
    let day = e - 2447 * f / 80;
    let g = f / 11;
    let month = f + 2 - 12 * g;
    let year = 100 * (b - 49) + d + g;
    (year as i32, month as u32, day as u32)
}
