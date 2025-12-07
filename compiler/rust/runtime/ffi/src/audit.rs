use crate::{BridgeAuditMetadata, BridgeReturnAuditMetadata, Span};
use serde_json::{json, Map, Value};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// 監査ログの単一エントリ。
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub event: String,
    pub payload: Value,
    pub metadata: Map<String, Value>,
    pub timestamp: SystemTime,
}

impl AuditEntry {
    fn with_metadata(
        event: impl Into<String>,
        payload: Value,
        metadata: Map<String, Value>,
    ) -> Self {
        Self {
            event: event.into(),
            payload,
            metadata,
            timestamp: SystemTime::now(),
        }
    }

    fn as_json(&self) -> Value {
        json!({
            "event": self.event,
            "timestamp": self
                .timestamp
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or_default(),
            "payload": self.payload,
            "metadata": self.metadata,
        })
    }
}

/// 監査ログを蓄積する sink。
#[derive(Clone, Debug)]
pub struct AuditSink {
    inner: Arc<Mutex<Vec<AuditEntry>>>,
}

impl AuditSink {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn log(
        &self,
        event: impl Into<String>,
        payload: Value,
        metadata: Map<String, Value>,
    ) -> Result<(), AuditError> {
        let entry = AuditEntry::with_metadata(event, payload, metadata);
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| AuditError::Transport("audit sink がロックできません".into()))?;
        guard.push(entry);
        Ok(())
    }

    pub fn entries(&self) -> Vec<AuditEntry> {
        let guard = self
            .inner
            .lock()
            .expect("audit sink mutex を取得できません");
        guard.clone()
    }
}

/// 監査用コンテキスト。
#[derive(Clone, Debug)]
pub struct AuditContext {
    domain: String,
    subject: String,
    metadata: Map<String, Value>,
    sink: AuditSink,
}

impl AuditContext {
    pub fn new(
        domain: impl Into<String>,
        subject: impl Into<String>,
        sink: AuditSink,
    ) -> Result<Self, AuditError> {
        let domain = domain.into();
        let subject = subject.into();
        if domain.is_empty() || subject.is_empty() {
            return Err(AuditError::PolicyViolation(
                "domain/subject は空にできません".into(),
            ));
        }
        Ok(Self {
            domain,
            subject,
            metadata: Map::new(),
            sink,
        })
    }

    pub fn with_metadata(mut self, metadata: Map<String, Value>) -> Self {
        self.metadata.extend(metadata);
        self
    }

    pub fn log(&self, event: impl Into<String>, payload: Value) -> Result<(), AuditError> {
        let event = event.into();
        let mut metadata = self.build_metadata();
        metadata.insert("event".into(), Value::String(event.clone()));
        self.sink.log(event, payload, metadata)
    }

    pub fn log_with_span(
        &self,
        event: impl Into<String>,
        span: Span,
        payload: Value,
    ) -> Result<(), AuditError> {
        let event = event.into();
        let mut metadata = self.build_metadata();
        metadata.insert("span".into(), span_to_value(span));
        self.sink.log(event, payload, metadata)
    }

    pub fn log_bridge_metadata(
        &self,
        event: impl Into<String>,
        bridge: &BridgeAuditMetadata<'_>,
        payload: Value,
    ) -> Result<(), AuditError> {
        let event = event.into();
        let mut metadata = self.build_metadata();
        add_bridge_metadata(&mut metadata, bridge);
        self.sink.log(event, payload, metadata)
    }

    fn build_metadata(&self) -> Map<String, Value> {
        let mut metadata = self.metadata.clone();
        metadata.insert("domain".into(), Value::String(self.domain.clone()));
        metadata.insert("subject".into(), Value::String(self.subject.clone()));
        metadata
    }
}

fn span_to_value(span: Span) -> Value {
    json!({ "start": span.start, "end": span.end, "length": span.len() })
}

fn add_bridge_metadata(metadata: &mut Map<String, Value>, bridge: &BridgeAuditMetadata<'_>) {
    metadata.insert(
        "bridge.status".into(),
        Value::String(bridge.status.as_str().to_string()),
    );
    metadata.insert(
        "bridge.ownership".into(),
        Value::String(bridge.ownership.as_str().to_string()),
    );
    metadata.insert("bridge.span".into(), span_to_value(bridge.span));
    metadata.insert(
        "bridge.target".into(),
        Value::String(bridge.target.to_string()),
    );
    metadata.insert("bridge.arch".into(), Value::String(bridge.arch.to_string()));
    metadata.insert(
        "bridge.platform".into(),
        Value::String(bridge.platform.to_string()),
    );
    metadata.insert("bridge.abi".into(), Value::String(bridge.abi.to_string()));
    metadata.insert(
        "bridge.expected_abi".into(),
        Value::String(bridge.expected_abi.to_string()),
    );
    metadata.insert(
        "bridge.symbol".into(),
        Value::String(bridge.symbol.to_string()),
    );
    metadata.insert(
        "bridge.extern_symbol".into(),
        Value::String(bridge.extern_symbol.to_string()),
    );
    metadata.insert(
        "bridge.extern_name".into(),
        Value::String(bridge.extern_name.to_string()),
    );
    metadata.insert(
        "bridge.link_name".into(),
        Value::String(bridge.link_name.to_string()),
    );
    metadata.insert(
        "bridge.return".into(),
        bridge_return_to_value(&bridge.return_info),
    );
}

fn bridge_return_to_value(return_info: &BridgeReturnAuditMetadata<'_>) -> Value {
    json!({
        "ownership": return_info.ownership.as_str(),
        "status": return_info.status,
        "wrap": return_info.wrap,
        "release_handler": return_info.release_handler,
        "rc_adjustment": return_info.rc_adjustment,
    })
}

/// 監査に失敗した場合のエラー。
#[derive(Debug, Clone)]
pub enum AuditError {
    Transport(String),
    Encoding(String),
    PolicyViolation(String),
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditError::Transport(msg) => write!(f, "Audit transport error: {}", msg),
            AuditError::Encoding(msg) => write!(f, "Audit encoding error: {}", msg),
            AuditError::PolicyViolation(msg) => write!(f, "Audit policy violation: {}", msg),
        }
    }
}

impl std::error::Error for AuditError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn audit_context_logs_entries() {
        let sink = AuditSink::new();
        let ctx = AuditContext::new("ffi", "call_fn", sink.clone()).unwrap();
        ctx.log("ffi.start", json!({ "status": "ok" })).unwrap();

        let entries = sink.entries();
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.event, "ffi.start");
        assert_eq!(entry.payload["status"], "ok");
        assert_eq!(entry.metadata["domain"], "ffi");
        assert_eq!(entry.metadata["subject"], "call_fn");
    }

    #[test]
    fn log_with_span_includes_span() {
        let sink = AuditSink::new();
        let ctx = AuditContext::new("ffi", "span", sink.clone()).unwrap();
        let span = Span::new(2, 5);
        ctx.log_with_span("ffi.span", span, json!({"ok": true}))
            .unwrap();

        let entry = sink.entries().into_iter().next().unwrap();
        assert_eq!(entry.metadata["span"]["start"], 2);
        assert_eq!(entry.metadata["span"]["length"], 3);
    }
}
