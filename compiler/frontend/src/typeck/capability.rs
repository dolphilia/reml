use std::fmt;
use std::str::FromStr;

use serde::Serialize;
use smol_str::SmolStr;

use super::env::StageId;
use crate::span::Span;

/// Reml が保持する効果と Capability の対応表。
struct CapabilityPattern {
    prefix: &'static str,
    id: &'static str,
}

impl CapabilityPattern {
    const fn new(prefix: &'static str, id: &'static str) -> Self {
        Self { prefix, id }
    }
}

const CAPABILITY_PATTERNS: &[CapabilityPattern] = &[
    CapabilityPattern::new("core.io.", "io"),
    CapabilityPattern::new("core.file.", "io"),
    CapabilityPattern::new("core.fs.", "io"),
    CapabilityPattern::new("core.time.", "time"),
    CapabilityPattern::new("core.text.", "unicode"),
    CapabilityPattern::new("core.process.", "process"),
    CapabilityPattern::new("core.thread.", "thread"),
    CapabilityPattern::new("core.system.", "syscall"),
    CapabilityPattern::new("core.memory.", "memory"),
    CapabilityPattern::new("core.signal.", "signal"),
    CapabilityPattern::new("core.hardware.", "hardware"),
    CapabilityPattern::new("core.realtime.", "realtime"),
    CapabilityPattern::new("core.diagnostics.audit_ctx.", "audit"),
    CapabilityPattern::new("core.security.", "security"),
    CapabilityPattern::new("core.trace.", "trace"),
    CapabilityPattern::new("core.debug.", "debug"),
    CapabilityPattern::new("core.collection.", "mem"),
];

const SPECIAL_CAPABILITIES: &[(&str, &str)] = &[
    ("panic", "panic"),
    ("unsafe", "unsafe"),
    ("ffi", "ffi"),
    ("runtime", "runtime"),
    ("metrics", "metrics"),
    ("audit", "audit"),
    ("time", "time"),
];

/// AST から抽出した効果の使用情報。
pub struct EffectUsage {
    pub effect_name: String,
    pub span: Span,
}

impl EffectUsage {
    pub fn new(effect_name: String, span: Span) -> Self {
        Self { effect_name, span }
    }
}

/// Capability のメタ情報。
#[derive(Debug)]
pub struct CapabilityDescriptor {
    id: SmolStr,
    stage: StageId,
    user_defined: bool,
}

impl CapabilityDescriptor {
    pub fn id(&self) -> &SmolStr {
        &self.id
    }

    pub fn stage(&self) -> &StageId {
        &self.stage
    }

    pub fn is_user_defined(&self) -> bool {
        self.user_defined
    }

    pub fn resolve(effect_name: &str) -> Self {
        let trimmed = effect_name.trim();
        if trimmed.is_empty() {
            return Self::with_user_defined_id("unknown");
        }
        let normalized = trimmed.trim_start_matches(':').to_ascii_lowercase();
        let has_console_segment = normalized.contains("::")
            && normalized
                .split("::")
                .filter(|segment| !segment.is_empty())
                .any(|segment| segment == "console");
        if has_console_segment {
            return Self::with_stage(&normalized, StageId::beta());
        }
        let normalized_pattern = normalized.replace("::", ".");
        for pattern in CAPABILITY_PATTERNS {
            if normalized_pattern.starts_with(pattern.prefix) {
                return Self::with_id(pattern.id);
            }
        }
        for (key, id) in SPECIAL_CAPABILITIES {
            if normalized_pattern == *key {
                return Self::with_id(id);
            }
        }
        // Fallback: use first segment of the identifier as capability, but treat it as
        // user-defined and keep the runtime stage at `stable`.
        let fallback = normalized_pattern
            .split('.')
            .next()
            .unwrap_or_else(|| normalized_pattern.as_str())
            .trim();
        if fallback.is_empty() {
            Self::with_user_defined_id("unknown")
        } else {
            Self::with_user_defined_id(fallback)
        }
    }

    fn with_id(id: &str) -> Self {
        let stage = match id {
            "panic" | "unsafe" | "ffi" | "runtime" => StageId::experimental(),
            _ => StageId::beta(),
        };
        Self::with_stage(id, stage)
    }

    fn with_stage(id: &str, stage: StageId) -> Self {
        Self {
            id: SmolStr::new(id.to_ascii_lowercase()),
            stage,
            user_defined: false,
        }
    }

    fn with_user_defined_id(id: &str) -> Self {
        let normalized = id.trim();
        let label = if normalized.is_empty() {
            "unknown".to_string()
        } else {
            normalized.to_ascii_lowercase()
        };
        Self {
            id: SmolStr::new(label),
            stage: StageId::stable(),
            user_defined: true,
        }
    }
}

/// CLI やコンフィグから渡された Runtime Capability の情報。
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeCapability {
    id: SmolStr,
    stage: StageId,
}

impl RuntimeCapability {
    pub fn parse(value: &str) -> Option<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        let (id_part, stage_part) = if let Some(idx) = trimmed.find('@') {
            (&trimmed[..idx], trimmed[idx + 1..].trim())
        } else if let Some(idx) = trimmed.find(':') {
            (&trimmed[..idx], trimmed[idx + 1..].trim())
        } else {
            (trimmed, "")
        };
        let id = id_part.trim().to_ascii_lowercase();
        if id.is_empty() {
            return None;
        }
        let stage = StageId::from_str(stage_part).unwrap_or_else(|_| StageId::stable());
        Some(Self {
            id: SmolStr::new(id),
            stage,
        })
    }

    pub fn id(&self) -> &SmolStr {
        &self.id
    }

    pub fn stage(&self) -> &StageId {
        &self.stage
    }

    pub fn new(id: impl Into<SmolStr>, stage: StageId) -> Self {
        Self {
            id: id.into(),
            stage,
        }
    }
}

impl fmt::Display for RuntimeCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stage_label = self.stage().as_str();
        if stage_label.eq_ignore_ascii_case("stable") {
            write!(f, "{}", self.id())
        } else {
            write!(f, "{}@{}", self.id(), stage_label)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_capability_parsing_basic() {
        let cap = RuntimeCapability::parse("io").expect("should parse");
        assert_eq!(cap.id(), "io");
        assert_eq!(cap.stage().as_str(), "stable");

        let cap_with_stage = RuntimeCapability::parse("metrics@beta").expect("should parse");
        assert_eq!(cap_with_stage.id(), "metrics");
        assert_eq!(cap_with_stage.stage().as_str(), "beta");

        let cap_colon = RuntimeCapability::parse("audit:experimental").expect("should parse");
        assert_eq!(cap_colon.id(), "audit");
        assert_eq!(cap_colon.stage().as_str(), "experimental");
    }

    #[test]
    fn descriptor_resolves_prefixes() {
        let descriptor = CapabilityDescriptor::resolve("core.io.print");
        assert_eq!(descriptor.id().as_str(), "io");

        let descriptor2 = CapabilityDescriptor::resolve("core.system.syscall");
        assert_eq!(descriptor2.id().as_str(), "syscall");

        let descriptor3 = CapabilityDescriptor::resolve("custom.unknown");
        assert_eq!(descriptor3.id().as_str(), "custom");
    }
}
