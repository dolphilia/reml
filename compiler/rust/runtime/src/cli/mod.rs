//! Core.Cli 仕様の最小実装。
//! フラグ/引数/サブコマンドの解析と、診断・監査イベントの記録を行う。

use once_cell::sync::Lazy;
use serde_json::{Map as JsonMap, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::audit::{AuditEnvelope, AuditEvent};
use crate::prelude::ensure::{DiagnosticNote, DiagnosticSeverity, GuardDiagnostic};

const CLI_DIAGNOSTIC_CODE: &str = "cli.parse.failed";
const CLI_DOMAIN: &str = "cli";
const CLI_AUDIT_KIND: &str = "cli.parse";
const DEFAULT_CLI_NAME: &str = "cli";

static CLI_DIAGNOSTICS: Lazy<Mutex<Vec<GuardDiagnostic>>> = Lazy::new(|| Mutex::new(Vec::new()));
static CLI_AUDIT_EVENTS: Lazy<Mutex<Vec<AuditEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// CLI 仕様。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliSpec {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub entries: Vec<CliEntry>,
}

/// CLI 要素。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliEntry {
    Flag {
        name: String,
        help: String,
    },
    Arg {
        name: String,
        help: String,
    },
    Command {
        name: String,
        help: String,
        spec: CliSpec,
    },
}

/// 解析結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliValues {
    pub flags: BTreeSet<String>,
    pub args: BTreeMap<String, String>,
    pub command: Option<String>,
}

impl CliValues {
    fn new() -> Self {
        Self {
            flags: BTreeSet::new(),
            args: BTreeMap::new(),
            command: None,
        }
    }
}

/// ビルダー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliBuilder {
    name: String,
    version: Option<String>,
    description: Option<String>,
    entries: Vec<CliEntry>,
}

/// CLI 解析エラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliError {
    pub kind: CliErrorKind,
    pub message: String,
    pub hint: Option<String>,
}

impl CliError {
    pub fn new(kind: CliErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    fn into_diagnostic(&self) -> GuardDiagnostic {
        let mut extensions = JsonMap::new();
        let mut cli_payload = JsonMap::new();
        cli_payload.insert(
            "error_kind".into(),
            Value::String(self.kind.as_str().to_string()),
        );
        if let Some(hint) = &self.hint {
            cli_payload.insert("hint".into(), Value::String(hint.clone()));
        }
        extensions.insert("cli".into(), Value::Object(cli_payload));

        let notes = self
            .hint
            .as_ref()
            .map(|hint| vec![DiagnosticNote::plain(hint.clone())])
            .unwrap_or_default();

        let mut audit_metadata = JsonMap::new();
        audit_metadata.insert(
            "cli.error.kind".into(),
            Value::String(self.kind.as_str().to_string()),
        );

        GuardDiagnostic {
            code: CLI_DIAGNOSTIC_CODE,
            domain: CLI_DOMAIN,
            severity: DiagnosticSeverity::Error,
            message: self.message.clone(),
            notes,
            extensions,
            audit_metadata,
        }
    }
}

/// CLI 解析エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliErrorKind {
    MissingArgument,
    UnknownFlag,
    UnknownCommand,
    ValueParseFailed,
}

impl CliErrorKind {
    fn as_str(&self) -> &'static str {
        match self {
            CliErrorKind::MissingArgument => "missing_argument",
            CliErrorKind::UnknownFlag => "unknown_flag",
            CliErrorKind::UnknownCommand => "unknown_command",
            CliErrorKind::ValueParseFailed => "value_parse_failed",
        }
    }
}

/// ビルダーを生成する。
pub fn builder() -> CliBuilder {
    CliBuilder {
        name: DEFAULT_CLI_NAME.to_string(),
        version: None,
        description: None,
        entries: Vec::new(),
    }
}

/// フラグを追加する。
pub fn flag(
    mut builder: CliBuilder,
    name: impl Into<String>,
    help: impl Into<String>,
) -> CliBuilder {
    builder.entries.push(CliEntry::Flag {
        name: name.into(),
        help: help.into(),
    });
    builder
}

/// 引数を追加する。
pub fn arg(
    mut builder: CliBuilder,
    name: impl Into<String>,
    help: impl Into<String>,
) -> CliBuilder {
    builder.entries.push(CliEntry::Arg {
        name: name.into(),
        help: help.into(),
    });
    builder
}

/// サブコマンドを追加する。
pub fn command(
    mut builder: CliBuilder,
    name: impl Into<String>,
    help: impl Into<String>,
    spec: CliSpec,
) -> CliBuilder {
    builder.entries.push(CliEntry::Command {
        name: name.into(),
        help: help.into(),
        spec,
    });
    builder
}

/// 仕様を構築する。
pub fn build(builder: CliBuilder) -> CliSpec {
    CliSpec {
        name: builder.name,
        version: builder.version,
        description: builder.description,
        entries: builder.entries,
    }
}

/// CLI を解析する。
pub fn parse(spec: CliSpec, argv: Vec<String>) -> Result<CliValues, CliError> {
    let mut values = CliValues::new();
    let mut index = CliSpecIndex::new(&spec);
    let mut arg_cursor = 0usize;
    let mut command_name = None;

    for token in argv {
        let token = token.trim().to_string();
        if token.is_empty() {
            continue;
        }

        if let Some(flag_name) = token.strip_prefix("--") {
            let flag_name = flag_name.trim();
            if flag_name.is_empty() {
                return record_cli_error(CliError::new(
                    CliErrorKind::UnknownFlag,
                    "空のフラグ名は指定できません",
                ));
            }
            if index.flags.contains_key(flag_name) {
                values.flags.insert(flag_name.to_string());
                continue;
            }
            return record_cli_error(unknown_flag_error(flag_name, &index));
        }

        if token.starts_with('-') {
            return record_cli_error(unknown_flag_error(&token, &index));
        }

        if command_name.is_none() {
            if let Some(command_spec) = index.commands.get(&token) {
                command_name = Some(token.clone());
                values.command = command_name.clone();
                index = CliSpecIndex::new(command_spec);
                arg_cursor = 0;
                continue;
            }
        }

        if let Some(arg_name) = index.args.get(arg_cursor) {
            values.args.insert(arg_name.clone(), token);
            arg_cursor += 1;
            continue;
        }

        if command_name.is_none() && !index.commands.is_empty() {
            return record_cli_error(unknown_command_error(&token, &index));
        }

        return record_cli_error(CliError::new(
            CliErrorKind::ValueParseFailed,
            format!("不明な引数が指定されました: {token}"),
        ));
    }

    if let Some(arg_name) = index.args.get(arg_cursor) {
        return record_cli_error(missing_argument_error(arg_name));
    }

    record_cli_audit(&values);
    Ok(values)
}

/// フラグ有無を取得する。
pub fn get_flag(values: CliValues, name: impl AsRef<str>) -> bool {
    values.flags.contains(name.as_ref())
}

/// 引数値を取得する。
pub fn get_arg(values: CliValues, name: impl AsRef<str>) -> Option<String> {
    values.args.get(name.as_ref()).cloned()
}

/// 記録済みの CLI 診断を取得してクリアする。
pub fn take_cli_diagnostics() -> Vec<GuardDiagnostic> {
    let mut diagnostics = CLI_DIAGNOSTICS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let drained = diagnostics.clone();
    diagnostics.clear();
    drained
}

/// 記録済みの CLI 監査イベントを取得してクリアする。
pub fn take_cli_audit_events() -> Vec<AuditEvent> {
    let mut events = CLI_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let drained = events.clone();
    events.clear();
    drained
}

struct CliSpecIndex {
    flags: BTreeMap<String, String>,
    args: Vec<String>,
    commands: BTreeMap<String, CliSpec>,
}

impl CliSpecIndex {
    fn new(spec: &CliSpec) -> Self {
        let mut flags = BTreeMap::new();
        let mut args = Vec::new();
        let mut commands = BTreeMap::new();

        for entry in &spec.entries {
            match entry {
                CliEntry::Flag { name, help } => {
                    let label = name.trim();
                    if !label.is_empty() {
                        flags.insert(label.to_string(), help.clone());
                    }
                }
                CliEntry::Arg { name, .. } => {
                    let label = name.trim();
                    if !label.is_empty() {
                        args.push(label.to_string());
                    }
                }
                CliEntry::Command { name, spec, .. } => {
                    let label = name.trim();
                    if !label.is_empty() {
                        commands.insert(label.to_string(), spec.clone());
                    }
                }
            }
        }

        Self {
            flags,
            args,
            commands,
        }
    }
}

fn record_cli_error(error: CliError) -> Result<CliValues, CliError> {
    let diagnostic = error.into_diagnostic();
    let mut diagnostics = CLI_DIAGNOSTICS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    diagnostics.push(diagnostic);
    Err(error)
}

fn record_cli_audit(values: &CliValues) {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());
    let mut metadata = JsonMap::new();
    metadata.insert(
        "event.kind".into(),
        Value::String(CLI_AUDIT_KIND.to_string()),
    );
    metadata.insert("event.domain".into(), Value::String(CLI_DOMAIN.to_string()));
    metadata.insert(
        "cli.command".into(),
        values
            .command
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    metadata.insert(
        "cli.flags".into(),
        Value::Array(values.flags.iter().cloned().map(Value::String).collect()),
    );
    let args = values
        .args
        .iter()
        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
        .collect::<JsonMap<_, _>>();
    metadata.insert("cli.args".into(), Value::Object(args));
    let envelope = AuditEnvelope::from_parts(metadata, None, None, Some("core.cli".into()));
    let event = AuditEvent::new(timestamp, envelope);
    let mut events = CLI_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    events.push(event);
}

fn unknown_flag_error(flag_name: &str, index: &CliSpecIndex) -> CliError {
    let mut error = CliError::new(
        CliErrorKind::UnknownFlag,
        format!("未知のフラグが指定されました: {flag_name}"),
    );
    if !index.flags.is_empty() {
        let hints = index
            .flags
            .keys()
            .map(|name| format!("--{name}"))
            .collect::<Vec<_>>()
            .join(", ");
        error = error.with_hint(format!("利用可能なフラグ: {hints}"));
    }
    error
}

fn unknown_command_error(command: &str, index: &CliSpecIndex) -> CliError {
    let mut error = CliError::new(
        CliErrorKind::UnknownCommand,
        format!("未知のサブコマンドが指定されました: {command}"),
    );
    if !index.commands.is_empty() {
        let hints = index
            .commands
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        error = error.with_hint(format!("利用可能なサブコマンド: {hints}"));
    }
    error
}

fn missing_argument_error(name: &str) -> CliError {
    CliError::new(
        CliErrorKind::MissingArgument,
        format!("必須引数が不足しています: {name}"),
    )
}
