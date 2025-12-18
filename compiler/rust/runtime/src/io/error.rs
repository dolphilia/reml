use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic};
use crate::prelude::iter::EffectLabels;
use serde_json::{Map, Number, Value};

use super::{attach_bridge_stage_metadata, context::GlobStats, BufferStats, IoContext, WatchStats};

/// IO 操作共通の結果型。
pub type IoResult<T> = Result<T, IoError>;

/// Core.IO 互換エラー。
#[derive(Debug, Clone)]
pub struct IoError {
    kind: IoErrorKind,
    message: String,
    path: Option<PathBuf>,
    context: Option<IoContext>,
    platform: Option<&'static str>,
    feature: Option<String>,
}

impl IoError {
    pub fn new(kind: IoErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            path: None,
            context: None,
            platform: None,
            feature: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        let path_buf = path.into();
        if let Some(context) = self.context.as_mut() {
            context.set_path(path_buf.clone());
        }
        self.path = Some(path_buf);
        self
    }

    pub fn with_context(mut self, context: IoContext) -> Self {
        if self.path.is_none() {
            if let Some(path) = context.path() {
                self.path = Some(path.to_path_buf());
            }
        }
        self.context = Some(context);
        self
    }

    pub fn with_platform(mut self, platform: &'static str) -> Self {
        self.platform = Some(platform);
        self
    }

    pub fn with_feature(mut self, feature: impl Into<String>) -> Self {
        self.feature = Some(feature.into());
        self
    }

    pub fn map_context<F>(mut self, f: F) -> Self
    where
        F: FnOnce(IoContext) -> IoContext,
    {
        if let Some(context) = self.context.take() {
            self.context = Some(f(context));
        }
        self
    }

    pub fn kind(&self) -> IoErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn context(&self) -> Option<&IoContext> {
        self.context.as_ref()
    }

    pub fn from_std(error: std::io::Error, context: IoContext) -> Self {
        let kind = IoErrorKind::from(error.kind());
        IoError::new(kind, error.to_string()).with_context(context)
    }
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for IoError {}

/// 仕様に沿った IO エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    InvalidInput,
    TimedOut,
    WriteZero,
    Interrupted,
    UnexpectedEof,
    OutOfMemory,
    SecurityViolation,
    UnsupportedPlatform,
}

impl From<std::io::ErrorKind> for IoErrorKind {
    fn from(kind: std::io::ErrorKind) -> Self {
        use std::io::ErrorKind as Std;
        match kind {
            Std::NotFound => IoErrorKind::NotFound,
            Std::PermissionDenied => IoErrorKind::PermissionDenied,
            Std::ConnectionRefused | Std::ConnectionReset | Std::ConnectionAborted => {
                IoErrorKind::ConnectionRefused
            }
            Std::BrokenPipe | Std::NotConnected | Std::AddrInUse | Std::AddrNotAvailable => {
                IoErrorKind::InvalidInput
            }
            Std::TimedOut => IoErrorKind::TimedOut,
            Std::WriteZero => IoErrorKind::WriteZero,
            Std::Interrupted => IoErrorKind::Interrupted,
            Std::UnexpectedEof => IoErrorKind::UnexpectedEof,
            Std::OutOfMemory => IoErrorKind::OutOfMemory,
            Std::Unsupported => IoErrorKind::UnsupportedPlatform,
            Std::WouldBlock | Std::InvalidInput | Std::InvalidData => IoErrorKind::InvalidInput,
            _ => IoErrorKind::InvalidInput,
        }
    }
}

impl IoErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            IoErrorKind::NotFound => "not_found",
            IoErrorKind::PermissionDenied => "permission_denied",
            IoErrorKind::ConnectionRefused => "connection_refused",
            IoErrorKind::InvalidInput => "invalid_input",
            IoErrorKind::TimedOut => "timed_out",
            IoErrorKind::WriteZero => "write_zero",
            IoErrorKind::Interrupted => "interrupted",
            IoErrorKind::UnexpectedEof => "unexpected_eof",
            IoErrorKind::OutOfMemory => "out_of_memory",
            IoErrorKind::SecurityViolation => "security_violation",
            IoErrorKind::UnsupportedPlatform => "unsupported_platform",
        }
    }

    fn default_code(&self) -> &'static str {
        match self {
            IoErrorKind::NotFound => "core.io.not_found",
            IoErrorKind::PermissionDenied => "core.io.permission_denied",
            IoErrorKind::ConnectionRefused => "core.io.connection_refused",
            IoErrorKind::InvalidInput => "core.io.invalid_input",
            IoErrorKind::TimedOut => "core.io.timed_out",
            IoErrorKind::WriteZero => "core.io.write_zero",
            IoErrorKind::Interrupted => "core.io.interrupted",
            IoErrorKind::UnexpectedEof => "core.io.unexpected_eof",
            IoErrorKind::OutOfMemory => "core.io.out_of_memory",
            IoErrorKind::SecurityViolation => "core.io.security_violation",
            IoErrorKind::UnsupportedPlatform => "core.io.unsupported_platform",
        }
    }
}

impl IntoDiagnostic for IoError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let IoError {
            kind,
            message,
            path,
            context,
            platform,
            feature,
        } = self;

        let context_ref = context.as_ref();
        let code = derive_diagnostic_code(kind, context_ref);

        let resolved_path = path
            .as_ref()
            .map(path_to_string)
            .or_else(|| context_ref.and_then(|ctx| ctx.path()).map(path_to_string));

        let mut io_extensions = Map::new();
        io_extensions.insert("kind".into(), Value::String(kind.as_str().into()));
        if let Some(ref path_value) = resolved_path {
            io_extensions.insert("path".into(), Value::String(path_value.clone()));
        }
        if let Some(ctx) = context_ref {
            io_extensions.insert(
                "operation".into(),
                Value::String(ctx.operation().to_string()),
            );
            if let Some(capability) = ctx.capability() {
                io_extensions.insert("capability".into(), Value::String(capability.into()));
            }
            if let Some(bytes) = ctx.bytes_processed() {
                io_extensions.insert("bytes_processed".into(), Value::Number(Number::from(bytes)));
            }
            if let Some(buffer) = ctx.buffer() {
                let buffer_map = encode_buffer_stats(buffer);
                io_extensions.insert("buffer".into(), Value::Object(buffer_map));
            }
            if let Some(watch) = ctx.watch_stats() {
                let watch_map = encode_watch_stats(watch);
                io_extensions.insert("watch".into(), Value::Object(watch_map));
            }
            if let Some(glob) = ctx.glob() {
                let glob_map = encode_glob_stats(glob);
                io_extensions.insert("glob".into(), Value::Object(glob_map));
            }
            if let Ok(timestamp_value) = serde_json::to_value(ctx.timestamp()) {
                io_extensions.insert("timestamp".into(), timestamp_value);
            }
        }
        if let Some(platform_value) = platform {
            io_extensions.insert("platform".into(), Value::String(platform_value.into()));
        }
        if let Some(feature_value) = feature.as_ref() {
            io_extensions.insert("feature".into(), Value::String(feature_value.clone()));
        }

        let mut extensions = Map::new();
        extensions.insert("io".into(), Value::Object(io_extensions));
        if let Some(ctx) = context_ref {
            let effects_map = encode_effect_labels(ctx.effects());
            extensions.insert("effects".into(), Value::Object(effects_map));
        }
        extensions.insert("message".into(), Value::String(message.clone()));

        let mut audit_metadata = Map::new();
        audit_metadata.insert("io.error.kind".into(), Value::String(kind.as_str().into()));
        if let Some(path_value) = resolved_path {
            audit_metadata.insert("io.path".into(), Value::String(path_value));
        }
        if let Some(ctx) = context_ref {
            audit_metadata.insert("io.operation".into(), Value::String(ctx.operation().into()));
            if let Some(capability) = ctx.capability() {
                audit_metadata.insert("io.capability".into(), Value::String(capability.into()));
                attach_bridge_stage_metadata(capability, &mut audit_metadata);
            }
            if let Some(bytes) = ctx.bytes_processed() {
                audit_metadata.insert(
                    "io.bytes_processed".into(),
                    Value::Number(Number::from(bytes)),
                );
            }
            if let Ok(timestamp_value) = serde_json::to_value(ctx.timestamp()) {
                audit_metadata.insert("io.timestamp".into(), timestamp_value);
            }
            if let Some(buffer) = ctx.buffer() {
                let buffer_map = encode_buffer_stats(buffer);
                for (key, value) in buffer_map {
                    audit_metadata.insert(format!("io.buffer.{key}"), value);
                }
            }
            if let Some(watch) = ctx.watch_stats() {
                let watch_map = encode_watch_stats(watch);
                for (key, value) in watch_map {
                    audit_metadata.insert(format!("io.watch.{key}"), value);
                }
            }
            if let Some(glob) = ctx.glob() {
                let glob_map = encode_glob_stats(glob);
                for (key, value) in glob_map {
                    audit_metadata.insert(format!("io.glob.{key}"), value);
                }
            }
            let effects_map = encode_effect_labels(ctx.effects());
            for (key, value) in effects_map {
                audit_metadata.insert(format!("io.effects.{key}"), value);
            }
        }
        if let Some(platform_value) = platform {
            audit_metadata.insert("io.platform".into(), Value::String(platform_value.into()));
        }
        if let Some(feature_value) = feature {
            audit_metadata.insert("io.feature".into(), Value::String(feature_value));
        }

        let diag_message = format_diagnostic_message(kind, context_ref, &message);

        GuardDiagnostic {
            code,
            domain: "runtime",
            severity: DiagnosticSeverity::Error,
            message: diag_message,
            notes: Vec::new(),
            extensions,
            audit_metadata,
        }
    }
}

const READ_ERROR_CODE: &str = "core.io.read_error";
const WRITE_ERROR_CODE: &str = "core.io.write_error";
const BUFFERED_READ_ERROR_CODE: &str = "core.io.read_error.buffered";
const WATCHER_ERROR_CODE: &str = "core.io.watcher_error";
const PATH_GLOB_ERROR_CODE: &str = "core.path.glob.io_error";

fn derive_diagnostic_code(kind: IoErrorKind, context: Option<&IoContext>) -> &'static str {
    if let Some(ctx) = context {
        if let Some(code) = operation_diagnostic_code(ctx.operation()) {
            return code;
        }
    }
    kind.default_code()
}

fn operation_diagnostic_code(operation: &str) -> Option<&'static str> {
    if operation == "path.glob" {
        return Some(PATH_GLOB_ERROR_CODE);
    }
    if operation.contains("watch") {
        return Some(WATCHER_ERROR_CODE);
    }
    if operation.contains("buffer") && operation.contains("read") {
        return Some(BUFFERED_READ_ERROR_CODE);
    }
    if operation.contains("write") || matches!(operation, "flush" | "sync_all" | "sync_data") {
        return Some(WRITE_ERROR_CODE);
    }
    if operation.contains("read") || operation == "with_reader" {
        return Some(READ_ERROR_CODE);
    }
    None
}

fn format_diagnostic_message(
    kind: IoErrorKind,
    context: Option<&IoContext>,
    message: &str,
) -> String {
    if let Some(ctx) = context {
        format!("Core.IO {} failed: {}", ctx.operation(), message)
    } else {
        format!("Core.IO {} error: {}", kind.as_str(), message)
    }
}

fn encode_effect_labels(labels: EffectLabels) -> Map<String, Value> {
    let mut effects = Map::new();
    effects.insert("mem".into(), Value::Bool(labels.mem));
    effects.insert("mutating".into(), Value::Bool(labels.mutating));
    effects.insert("debug".into(), Value::Bool(labels.debug));
    effects.insert("async_pending".into(), Value::Bool(labels.async_pending));
    effects.insert("audit".into(), Value::Bool(labels.audit));
    effects.insert("cell".into(), Value::Bool(labels.cell));
    effects.insert("rc".into(), Value::Bool(labels.rc));
    effects.insert("unicode".into(), Value::Bool(labels.unicode));
    effects.insert("io".into(), Value::Bool(labels.io));
    effects.insert("io_blocking".into(), Value::Bool(labels.io_blocking));
    effects.insert("io_async".into(), Value::Bool(labels.io_async));
    effects.insert("security".into(), Value::Bool(labels.security));
    effects.insert("transfer".into(), Value::Bool(labels.transfer));
    effects.insert("fs_sync".into(), Value::Bool(labels.fs_sync));
    effects.insert(
        "mem_bytes".into(),
        Value::Number(Number::from(labels.mem_bytes as u64)),
    );
    effects.insert(
        "predicate_calls".into(),
        Value::Number(Number::from(labels.predicate_calls as u64)),
    );
    effects.insert(
        "rc_ops".into(),
        Value::Number(Number::from(labels.rc_ops as u64)),
    );
    effects.insert("time".into(), Value::Bool(labels.time));
    effects.insert(
        "time_calls".into(),
        Value::Number(Number::from(labels.time_calls as u64)),
    );
    effects.insert(
        "io_blocking_calls".into(),
        Value::Number(Number::from(labels.io_blocking_calls as u64)),
    );
    effects.insert(
        "io_async_calls".into(),
        Value::Number(Number::from(labels.io_async_calls as u64)),
    );
    effects.insert(
        "fs_sync_calls".into(),
        Value::Number(Number::from(labels.fs_sync_calls as u64)),
    );
    effects.insert(
        "security_events".into(),
        Value::Number(Number::from(labels.security_events as u64)),
    );
    effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtime::bridge::RuntimeBridgeRegistry,
        stage::{StageId, StageRequirement},
    };

    #[test]
    fn audit_metadata_carries_bridge_stage_details() {
        let _guard = RuntimeBridgeRegistry::test_lock().lock().unwrap();
        const CAP: &str = "io.fs.error_test";
        RuntimeBridgeRegistry::global().clear();
        super::super::record_bridge_stage_probe(
            CAP,
            StageRequirement::Exact(StageId::Stable),
            StageId::Stable,
        );
        let ctx = IoContext::new("with_reader").with_capability(CAP);
        let diag = IoError::new(IoErrorKind::PermissionDenied, "denied")
            .with_context(ctx)
            .into_diagnostic();
        assert_eq!(
            diag.audit_metadata
                .get("bridge.stage.required")
                .and_then(Value::as_str),
            Some("stable")
        );
        assert_eq!(
            diag.audit_metadata
                .get("bridge.capability")
                .and_then(Value::as_str),
            Some(CAP)
        );
    }
}

fn encode_buffer_stats(stats: &BufferStats) -> Map<String, Value> {
    let mut buffer = Map::new();
    buffer.insert(
        "capacity".into(),
        Value::Number(Number::from(stats.capacity() as u64)),
    );
    buffer.insert(
        "fill".into(),
        Value::Number(Number::from(stats.fill() as u64)),
    );
    if let Ok(timestamp_value) = serde_json::to_value(stats.last_fill_timestamp()) {
        buffer.insert("last_fill_timestamp".into(), timestamp_value);
    }
    buffer
}

fn encode_glob_stats(stats: &GlobStats) -> Map<String, Value> {
    let mut glob = Map::new();
    if !stats.pattern().is_empty() {
        glob.insert("pattern".into(), Value::String(stats.pattern().to_string()));
    }
    if let Some(offending) = stats.offending_path() {
        glob.insert(
            "offending_path".into(),
            Value::String(offending.to_string()),
        );
    }
    glob
}

fn encode_watch_stats(stats: &WatchStats) -> Map<String, Value> {
    let mut watch = Map::new();
    watch.insert(
        "queue_size".into(),
        Value::Number(Number::from(stats.queue_size() as u64)),
    );
    watch.insert(
        "delay_ns".into(),
        Value::Number(Number::from(stats.delay_ns())),
    );
    watch
}

fn path_to_string(path: &PathBuf) -> String {
    path.to_string_lossy().into_owned()
}
