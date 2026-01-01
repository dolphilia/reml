use serde_json::{Map as JsonMap, Number, Value};

use crate::audit::AuditEnvelope;
use crate::runtime::Signal;

use super::process::{Command, ExitStatus, ProcessId};
use super::signal::SignalDetail;

const SYSTEM_EVENT_DOMAIN: &str = "core.system";
const RAW_CODE_MASKED: &str = "masked";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessAuditEvent {
    Spawn,
    Wait,
    Kill,
}

impl ProcessAuditEvent {
    fn as_str(self) -> &'static str {
        match self {
            ProcessAuditEvent::Spawn => "process.spawn",
            ProcessAuditEvent::Wait => "process.wait",
            ProcessAuditEvent::Kill => "process.kill",
        }
    }
}

pub struct ProcessAuditInfo<'a> {
    pub event: ProcessAuditEvent,
    pub pid: Option<ProcessId>,
    pub command: Option<&'a Command>,
    pub exit_status: Option<ExitStatus>,
    pub signal: Option<Signal>,
}

pub fn insert_process_audit_metadata(envelope: &mut AuditEnvelope, info: &ProcessAuditInfo<'_>) {
    let command_value = info
        .command
        .map(format_command)
        .map(Value::String)
        .unwrap_or(Value::Null);

    envelope
        .metadata
        .insert("event.kind".into(), Value::String(info.event.as_str().into()));
    envelope.metadata.insert(
        "event.domain".into(),
        Value::String(SYSTEM_EVENT_DOMAIN.into()),
    );
    envelope
        .metadata
        .insert("process.command".into(), command_value);
    insert_optional_i64(&mut envelope.metadata, "process.pid", info.pid);
    insert_optional_i64(
        &mut envelope.metadata,
        "process.exit_status",
        info.exit_status,
    );
    if let Some(signal) = info.signal {
        insert_optional_i64(&mut envelope.metadata, "process.signal", Some(signal));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalAuditEvent {
    Send,
    Wait,
    Raise,
}

impl SignalAuditEvent {
    fn as_str(self) -> &'static str {
        match self {
            SignalAuditEvent::Send => "signal.send",
            SignalAuditEvent::Wait => "signal.wait",
            SignalAuditEvent::Raise => "signal.raise",
        }
    }
}

pub struct SignalAuditInfo<'a> {
    pub event: SignalAuditEvent,
    pub signal: Signal,
    pub target_pid: Option<ProcessId>,
    pub detail: Option<&'a SignalDetail>,
    /// `signal.raw_code = "allow"` のときのみ数値を出力する。
    pub raw_code_policy: Option<&'a str>,
}

pub fn insert_signal_audit_metadata(envelope: &mut AuditEnvelope, info: &SignalAuditInfo<'_>) {
    envelope
        .metadata
        .insert("event.kind".into(), Value::String(info.event.as_str().into()));
    envelope.metadata.insert(
        "event.domain".into(),
        Value::String(SYSTEM_EVENT_DOMAIN.into()),
    );
    insert_optional_i64(
        &mut envelope.metadata,
        "signal.signal",
        Some(info.signal),
    );
    insert_optional_i64(&mut envelope.metadata, "signal.target_pid", info.target_pid);

    if let Some(detail) = info.detail {
        insert_optional_i64(
            &mut envelope.metadata,
            "signal.sender",
            Some(detail.info.sender),
        );
        insert_optional_i64(
            &mut envelope.metadata,
            "signal.source_pid",
            detail.source_pid,
        );
        envelope.metadata.insert(
            "signal.raw_code".into(),
            signal_raw_code_value(detail.raw_code, info.raw_code_policy),
        );
        if let Some(payload) = detail.payload.as_ref() {
            if let Ok(value) = serde_json::to_value(payload) {
                envelope.metadata.insert("signal.payload".into(), value);
            }
        }
        if let Some(timestamp) = detail.timestamp {
            if let Ok(value) = serde_json::to_value(timestamp) {
                envelope.metadata.insert("signal.timestamp".into(), value);
            }
        }
    } else {
        envelope
            .metadata
            .insert("signal.raw_code".into(), Value::Null);
    }
}

fn format_command(command: &Command) -> String {
    let mut parts = Vec::with_capacity(1 + command.args.len());
    parts.push(command.program.to_string_lossy().to_string());
    parts.extend(command.args.iter().cloned());
    parts.join(" ")
}

fn insert_optional_i64(map: &mut JsonMap<String, Value>, key: &str, value: Option<i64>) {
    let value = match value {
        Some(value) => Value::Number(Number::from(value)),
        None => Value::Null,
    };
    map.insert(key.into(), value);
}

fn signal_raw_code_value(raw_code: Option<i64>, raw_code_policy: Option<&str>) -> Value {
    let allow_raw_code = raw_code_policy
        .map(str::trim)
        .map(|value| value.eq_ignore_ascii_case("allow"))
        .unwrap_or(false);
    match raw_code {
        Some(code) if allow_raw_code => Value::Number(Number::from(code)),
        Some(_) => Value::String(RAW_CODE_MASKED.to_string()),
        None => Value::Null,
    }
}
