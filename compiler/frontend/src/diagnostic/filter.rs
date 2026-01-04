use serde_json::{json, Map, Value};

use super::{DiagnosticSeverity, FrontendDiagnostic};

/// Stage 情報に応じて実験的診断の Severity を降格させる。
pub fn apply_experimental_stage_policy(
    diagnostic: &mut FrontendDiagnostic,
    extensions: &Map<String, Value>,
    ack_experimental: bool,
) {
    if should_downgrade_experimental(ack_experimental, extensions)
        && diagnostic.severity_or_default() == DiagnosticSeverity::Error
    {
        diagnostic.set_severity(DiagnosticSeverity::Warning);
    }
}

/// Stage 拡張から実験扱いかどうかを判定する。
pub fn should_downgrade_experimental(
    ack_experimental: bool,
    extensions: &Map<String, Value>,
) -> bool {
    if ack_experimental {
        return false;
    }
    stage_map_is_experimental(extension_stage_map(extensions, "effects"))
        || stage_map_is_experimental(extension_stage_map(extensions, "capability"))
        || stage_map_is_experimental(extension_stage_map(extensions, "bridge"))
        || any_flat_stage_is_experimental(extensions)
}

fn extension_stage_map<'a>(
    extensions: &'a Map<String, Value>,
    namespace: &str,
) -> Option<&'a Map<String, Value>> {
    extensions
        .get(namespace)?
        .as_object()?
        .get("stage")?
        .as_object()
}

fn stage_map_is_experimental(stage_map: Option<&Map<String, Value>>) -> bool {
    let Some(map) = stage_map else {
        return false;
    };
    map.get("required")
        .and_then(Value::as_str)
        .map_or(false, is_experimental_label)
        || map
            .get("actual")
            .and_then(Value::as_str)
            .map_or(false, is_experimental_label)
}

fn any_flat_stage_is_experimental(extensions: &Map<String, Value>) -> bool {
    const STAGE_KEYS: &[&str] = &[
        "effect.stage.required",
        "effect.stage.actual",
        "effects.contract.stage.required",
        "effects.contract.stage.actual",
    ];
    STAGE_KEYS.iter().any(|key| {
        extensions
            .get(*key)
            .and_then(Value::as_str)
            .map_or(false, is_experimental_label)
    })
}

fn is_experimental_label(label: &str) -> bool {
    label.to_ascii_lowercase().contains("experimental")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SeverityRank {
    Error = 0,
    Warning = 1,
    Info = 2,
    Hint = 3,
}

impl SeverityRank {
    fn from_label(label: &str) -> Option<Self> {
        match label.to_ascii_lowercase().as_str() {
            "error" => Some(Self::Error),
            "warning" => Some(Self::Warning),
            "info" => Some(Self::Info),
            "hint" => Some(Self::Hint),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DiagnosticFilter {
    min_severity: SeverityRank,
    include_patterns: Vec<FilterPattern>,
    exclude_patterns: Vec<FilterPattern>,
}

impl Default for DiagnosticFilter {
    fn default() -> Self {
        Self {
            min_severity: SeverityRank::Hint,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
        }
    }
}

impl DiagnosticFilter {
    pub fn parse_assignment(previous: Option<Self>, assignment: &str) -> Result<Self, String> {
        let mut filter = previous.unwrap_or_default();
        let (key, value) = split_assignment(assignment)?;
        match key {
            "severity" => {
                filter.min_severity = SeverityRank::from_label(value).ok_or_else(|| {
                    format!("--diagnostic-filter severity は error|warning|info|hint のいずれかを指定してください (入力値: {value})")
                })?;
            }
            "include" => filter.include_patterns.push(FilterPattern::new(value)),
            "exclude" => filter.exclude_patterns.push(FilterPattern::new(value)),
            other => {
                return Err(format!(
                    "--diagnostic-filter `{other}` は未サポートのキーです"
                ))
            }
        }
        Ok(filter)
    }

    pub fn from_json(value: &Value) -> Result<Self, String> {
        let mut filter = Self::default();
        let obj = value
            .as_object()
            .ok_or_else(|| "--diagnostics.filter はオブジェクトである必要があります".to_string())?;
        if let Some(severity) = obj.get("severity").and_then(Value::as_str) {
            filter.min_severity = SeverityRank::from_label(severity).ok_or_else(|| {
                format!("diagnostics.filter.severity `{severity}` は解釈できません")
            })?;
        }
        parse_pattern_collection(obj.get("include"), &mut filter.include_patterns)?;
        parse_pattern_collection(obj.get("exclude"), &mut filter.exclude_patterns)?;
        Ok(filter)
    }

    pub fn to_value(&self) -> Value {
        let mut map = Map::new();
        map.insert(
            "severity".to_string(),
            json!(match self.min_severity {
                SeverityRank::Error => "error",
                SeverityRank::Warning => "warning",
                SeverityRank::Info => "info",
                SeverityRank::Hint => "hint",
            }),
        );
        if !self.include_patterns.is_empty() {
            map.insert(
                "include".to_string(),
                Value::Array(
                    self.include_patterns
                        .iter()
                        .map(|pattern| Value::String(pattern.raw.clone()))
                        .collect(),
                ),
            );
        }
        if !self.exclude_patterns.is_empty() {
            map.insert(
                "exclude".to_string(),
                Value::Array(
                    self.exclude_patterns
                        .iter()
                        .map(|pattern| Value::String(pattern.raw.clone()))
                        .collect(),
                ),
            );
        }
        Value::Object(map)
    }

    pub fn allows_value(&self, diag: &Value) -> bool {
        let severity = diag
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("error");
        let diag_rank = SeverityRank::from_label(severity).unwrap_or(SeverityRank::Error);
        if diag_rank > self.min_severity {
            return false;
        }
        let candidates = diagnostic_candidates(diag);
        if !self.include_patterns.is_empty()
            && !self.include_patterns.iter().any(|pattern| {
                candidates
                    .iter()
                    .any(|candidate| pattern.matches(candidate))
            })
        {
            return false;
        }
        if self.exclude_patterns.iter().any(|pattern| {
            candidates
                .iter()
                .any(|candidate| pattern.matches(candidate))
        }) {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug, Default)]
struct FilterPattern {
    raw: String,
    normalized: String,
}

impl FilterPattern {
    fn new(value: impl Into<String>) -> Self {
        let raw = value.into();
        Self {
            normalized: raw.to_ascii_lowercase(),
            raw,
        }
    }

    fn matches(&self, candidate: &str) -> bool {
        wildcard_match(&self.normalized, &candidate.to_ascii_lowercase())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditLevel {
    Off = 0,
    Error = 1,
    Warning = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl AuditLevel {
    fn from_label(label: &str) -> Option<Self> {
        match label.to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "error" => Some(Self::Error),
            "warning" => Some(Self::Warning),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            "trace" => Some(Self::Trace),
            _ => None,
        }
    }

    fn from_severity(label: &str) -> Self {
        match label.to_ascii_lowercase().as_str() {
            "warning" => Self::Warning,
            "info" => Self::Info,
            "hint" => Self::Debug,
            _ => Self::Error,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuditPolicy {
    level: AuditLevel,
    include_patterns: Vec<FilterPattern>,
    exclude_patterns: Vec<FilterPattern>,
    retention_days: Option<u32>,
    anonymize_pii: bool,
}

impl Default for AuditPolicy {
    fn default() -> Self {
        Self {
            level: AuditLevel::Warning,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            retention_days: None,
            anonymize_pii: false,
        }
    }
}

impl AuditPolicy {
    pub fn parse_assignment(previous: Option<Self>, assignment: &str) -> Result<Self, String> {
        let mut policy = previous.unwrap_or_default();
        let (key, value) = split_assignment(assignment)?;
        match key {
            "level" => {
                policy.level = AuditLevel::from_label(value).ok_or_else(|| {
                    format!(
                        "--audit-policy level は off|error|warning|info|debug|trace のいずれかです (入力: {value})"
                    )
                })?;
            }
            "include" | "include_patterns" => {
                policy.include_patterns.push(FilterPattern::new(value))
            }
            "exclude" | "exclude_patterns" => {
                policy.exclude_patterns.push(FilterPattern::new(value))
            }
            "retention_days" => {
                policy.retention_days = Some(value.parse::<u32>().map_err(|_| {
                    format!("--audit-policy retention_days `{value}` は整数ではありません")
                })?);
            }
            "anonymize_pii" => {
                policy.anonymize_pii = matches!(value, "1" | "true" | "on");
            }
            other => return Err(format!("--audit-policy `{other}` は未サポートのキーです")),
        }
        Ok(policy)
    }

    pub fn from_json(value: &Value) -> Result<Self, String> {
        let mut policy = Self::default();
        let obj = value
            .as_object()
            .ok_or_else(|| "--audit.policy はオブジェクトである必要があります".to_string())?;
        if let Some(level) = obj.get("level").and_then(Value::as_str) {
            policy.level = AuditLevel::from_label(level)
                .ok_or_else(|| format!("audit.policy.level `{level}` は解釈できません"))?;
        }
        parse_pattern_collection(obj.get("include"), &mut policy.include_patterns)?;
        parse_pattern_collection(obj.get("include_patterns"), &mut policy.include_patterns)?;
        parse_pattern_collection(obj.get("exclude"), &mut policy.exclude_patterns)?;
        parse_pattern_collection(obj.get("exclude_patterns"), &mut policy.exclude_patterns)?;
        if let Some(days) = obj.get("retention_days").and_then(Value::as_u64) {
            policy.retention_days = Some(days as u32);
        }
        if let Some(mask) = obj.get("anonymize_pii").and_then(Value::as_bool) {
            policy.anonymize_pii = mask;
        }
        Ok(policy)
    }

    pub fn to_value(&self) -> Value {
        let mut map = Map::new();
        map.insert(
            "level".to_string(),
            json!(match self.level {
                AuditLevel::Off => "off",
                AuditLevel::Error => "error",
                AuditLevel::Warning => "warning",
                AuditLevel::Info => "info",
                AuditLevel::Debug => "debug",
                AuditLevel::Trace => "trace",
            }),
        );
        if let Some(days) = self.retention_days {
            map.insert("retention_days".to_string(), json!(days));
        }
        if self.anonymize_pii {
            map.insert("anonymize_pii".to_string(), json!(true));
        }
        if !self.include_patterns.is_empty() {
            map.insert(
                "include_patterns".to_string(),
                Value::Array(
                    self.include_patterns
                        .iter()
                        .map(|pattern| Value::String(pattern.raw.clone()))
                        .collect(),
                ),
            );
        }
        if !self.exclude_patterns.is_empty() {
            map.insert(
                "exclude_patterns".to_string(),
                Value::Array(
                    self.exclude_patterns
                        .iter()
                        .map(|pattern| Value::String(pattern.raw.clone()))
                        .collect(),
                ),
            );
        }
        Value::Object(map)
    }

    pub fn apply(&self, diag: &mut Value) -> AuditEnforcement {
        let mut enforcement = AuditEnforcement::default();
        if !self.allows(diag) {
            enforcement.dropped = true;
            if let Some(obj) = diag.as_object_mut() {
                obj.insert("audit".to_string(), Value::Null);
                if let Some(meta) = obj
                    .entry("audit_metadata".to_string())
                    .or_insert_with(|| Value::Object(Map::new()))
                    .as_object_mut()
                {
                    meta.insert("audit.policy.dropped".to_string(), Value::Bool(true));
                }
            }
        }
        if self.anonymize_pii {
            enforcement.anonymized = true;
            if let Some(obj) = diag.as_object_mut() {
                if let Some(meta) = obj
                    .entry("audit_metadata".to_string())
                    .or_insert_with(|| Value::Object(Map::new()))
                    .as_object_mut()
                {
                    meta.insert("privacy.anonymized".to_string(), Value::Bool(true));
                }
                if let Some(audit) = obj
                    .entry("audit".to_string())
                    .or_insert_with(|| Value::Object(Map::new()))
                    .as_object_mut()
                {
                    let metadata_entry = audit
                        .entry("metadata".to_string())
                        .or_insert_with(|| Value::Object(Map::new()));
                    if let Some(metadata) = metadata_entry.as_object_mut() {
                        metadata.insert("privacy.redacted".to_string(), Value::Bool(true));
                    }
                }
            }
        }
        enforcement
    }

    pub fn allows(&self, diag: &Value) -> bool {
        if self.level == AuditLevel::Off {
            return false;
        }
        let severity = diag
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("error");
        let severity_level = AuditLevel::from_severity(severity);
        if self.level < severity_level {
            return false;
        }
        let candidates = diagnostic_candidates(diag);
        if !self.include_patterns.is_empty()
            && !self.include_patterns.iter().any(|pattern| {
                candidates
                    .iter()
                    .any(|candidate| pattern.matches(candidate))
            })
        {
            return false;
        }
        if self.exclude_patterns.iter().any(|pattern| {
            candidates
                .iter()
                .any(|candidate| pattern.matches(candidate))
        }) {
            return false;
        }
        true
    }

    pub fn anonymize_pii(&self) -> bool {
        self.anonymize_pii
    }
}

#[derive(Default)]
pub struct AuditEnforcement {
    pub dropped: bool,
    pub anonymized: bool,
}

fn parse_pattern_collection(
    value: Option<&Value>,
    target: &mut Vec<FilterPattern>,
) -> Result<(), String> {
    match value {
        None => Ok(()),
        Some(Value::Array(items)) => {
            for entry in items {
                if let Some(text) = entry.as_str() {
                    target.push(FilterPattern::new(text));
                } else {
                    return Err("フィルタパターンは文字列である必要があります".to_string());
                }
            }
            Ok(())
        }
        Some(Value::String(text)) => {
            target.push(FilterPattern::new(text));
            Ok(())
        }
        _ => Err("フィルタパターンは文字列または文字列配列として指定してください".to_string()),
    }
}

fn diagnostic_candidates(diag: &Value) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(code) = diag.get("code").and_then(Value::as_str) {
        candidates.push(code.to_ascii_lowercase());
    }
    if let Some(codes) = diag.get("codes").and_then(Value::as_array) {
        for entry in codes {
            if let Some(code) = entry.as_str() {
                candidates.push(code.to_ascii_lowercase());
            }
        }
    }
    if let Some(domain) = diag.get("domain").and_then(Value::as_str) {
        candidates.push(domain.to_ascii_lowercase());
    }
    candidates
}

fn split_assignment(spec: &str) -> Result<(&str, &str), String> {
    let mut parts = spec.splitn(2, '=');
    let key = parts
        .next()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "キーと値を `key=value` 形式で指定してください".to_string())?;
    let value = parts
        .next()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "値を含まない `key=value` 指定になっています".to_string())?;
    Ok((key, value))
}

fn wildcard_match(pattern: &str, candidate: &str) -> bool {
    let mut pat_bytes = Vec::with_capacity(pattern.len());
    let mut last_star = false;
    for byte in pattern.as_bytes() {
        if *byte == b'*' {
            if last_star {
                continue;
            }
            last_star = true;
        } else {
            last_star = false;
        }
        pat_bytes.push(*byte);
    }
    let cand_bytes = candidate.as_bytes();
    let (mut p_idx, mut c_idx, mut star_idx, mut match_idx) = (0usize, 0usize, None, 0usize);
    while c_idx < cand_bytes.len() {
        if p_idx < pat_bytes.len()
            && (pat_bytes[p_idx] == b'?' || pat_bytes[p_idx] == cand_bytes[c_idx])
        {
            p_idx += 1;
            c_idx += 1;
        } else if p_idx < pat_bytes.len() && pat_bytes[p_idx] == b'*' {
            star_idx = Some(p_idx);
            match_idx = c_idx;
            p_idx += 1;
        } else if let Some(star) = star_idx {
            p_idx = star + 1;
            match_idx += 1;
            c_idx = match_idx;
        } else {
            return false;
        }
    }
    while p_idx < pat_bytes.len() && pat_bytes[p_idx] == b'*' {
        p_idx += 1;
    }
    p_idx == pat_bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_pattern_matches_variations() {
        let pattern = FilterPattern::new("effects.*");
        assert!(pattern.matches("effects.contract.stage_mismatch"));
        assert!(!pattern.matches("parser.stage_mismatch"));
        let pattern = FilterPattern::new("parser.?rror");
        assert!(pattern.matches("parser.error"));
        assert!(!pattern.matches("parser.longerror"));
    }

    #[test]
    fn diagnostic_filter_enforces_severity() {
        let mut diag = json!({
            "severity": "info",
            "code": "demo.info",
            "domain": "parser"
        });
        let filter = DiagnosticFilter {
            min_severity: SeverityRank::Warning,
            ..DiagnosticFilter::default()
        };
        assert!(!filter.allows_value(&diag));
        diag["severity"] = json!("warning");
        assert!(filter.allows_value(&diag));
    }

    #[test]
    fn audit_policy_drops_when_level_lower() {
        let policy = AuditPolicy {
            level: AuditLevel::Error,
            ..AuditPolicy::default()
        };
        let mut diag = json!({
            "severity": "warning",
            "code": "demo.warning",
            "audit": { "metadata": {}},
            "audit_metadata": {}
        });
        let result = policy.apply(&mut diag);
        assert!(result.dropped);
        assert_eq!(diag["audit"], Value::Null);
    }
}
