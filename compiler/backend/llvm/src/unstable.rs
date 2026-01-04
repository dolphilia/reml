#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnstableKind {
    InlineAsm,
    LlvmIr,
}

impl UnstableKind {
    pub fn as_label(&self) -> &'static str {
        match self {
            UnstableKind::InlineAsm => "inline_asm",
            UnstableKind::LlvmIr => "llvm_ir",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnstableStatus {
    Enabled,
    Disabled,
}

#[derive(Clone, Debug)]
pub struct UnstableUse {
    pub function: String,
    pub kind: UnstableKind,
    pub payload: Option<String>,
    pub status: UnstableStatus,
}

impl UnstableUse {
    pub fn describe(&self) -> String {
        match &self.payload {
            Some(payload) => format!("{}:{}", self.kind.as_label(), payload),
            None => self.kind.as_label().to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UnstableRequest {
    pub kind: UnstableKind,
    pub payload: Option<String>,
}

pub fn parse_unstable_attribute(attr: &str) -> Option<UnstableRequest> {
    let trimmed = attr.trim();
    if let Some(rest) = trimmed.strip_prefix("unstable:inline_asm") {
        return Some(UnstableRequest {
            kind: UnstableKind::InlineAsm,
            payload: parse_payload(rest),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("unstable:llvm_ir") {
        return Some(UnstableRequest {
            kind: UnstableKind::LlvmIr,
            payload: parse_payload(rest),
        });
    }
    None
}

pub fn native_unstable_enabled() -> bool {
    cfg!(feature = "native-unstable")
}

fn parse_payload(rest: &str) -> Option<String> {
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        None
    } else if let Some(value) = trimmed.strip_prefix('=') {
        let value = value.trim();
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    } else {
        Some(trimmed.to_string())
    }
}
