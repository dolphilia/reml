//! Core.Path 抽象の最小実装。
//! 仕様に沿った Path/PathBuf/PathError を提供し、パス検証や正規化を
//! Rust Runtime から利用できるようにする。

use std::borrow::Cow;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::path::{Component, Path as StdPath, PathBuf as StdPathBuf};

use crate::prelude::{
    ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic},
    iter::EffectLabels,
};
use crate::text::Str;
use serde_json::{Map, Number, Value};

mod glob;
mod security;
mod string_utils;

pub use self::glob::glob;
pub use security::{
    is_safe_symlink, sandbox_path, validate_path, PathSecurityError, PathSecurityErrorKind,
    PathSecurityResult, SecurityPolicy,
};
pub use string_utils::{is_absolute_str, join_paths_str, normalize_path_str, relative_to};

/// Path 関連 API の結果型。
pub type PathResult<T> = Result<T, PathError>;

/// Core.Path の `PathBuf` に相当する所有型。
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct PathBuf {
    inner: StdPathBuf,
}

/// Core.Path の `Path` 参照型。
#[derive(Copy, Clone, Debug)]
pub struct Path<'a> {
    inner: &'a StdPath,
}

/// パス検証時のエラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathError {
    kind: PathErrorKind,
    message: String,
    invalid_input: Option<String>,
    origin: PathErrorOrigin,
    effects: Option<EffectLabels>,
}

/// パスエラーの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathErrorKind {
    Empty,
    NullByte,
    InvalidEncoding,
    UnsupportedPlatform,
    InvalidPattern,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathErrorOrigin {
    Generic,
    Glob(GlobContext),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GlobContext {
    pattern: Option<String>,
    offending_path: Option<String>,
}

impl GlobContext {
    fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: Some(pattern.into()),
            offending_path: None,
        }
    }
}

/// 文字列レベルでのパススタイル。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStyle {
    Native,
    Posix,
    Windows,
}

const PATH_GLOB_INVALID_PATTERN_CODE: &str = "core.path.glob.invalid_pattern";
const PATH_GLOB_IO_ERROR_CODE: &str = "core.path.glob.io_error";
const PATH_GLOB_INVALID_INPUT_CODE: &str = "core.path.glob.invalid_input";
const PATH_GLOB_UNSUPPORTED_PLATFORM_CODE: &str = "core.path.glob.unsupported_platform";
const PATH_GENERIC_ERROR_CODE: &str = "core.path.error";

impl PathBuf {
    /// 新しい空 PathBuf を生成する。
    pub fn new() -> Self {
        Self {
            inner: StdPathBuf::new(),
        }
    }

    /// 標準 PathBuf から Core.Path Buf へ変換する。
    pub fn from_std(inner: StdPathBuf) -> Self {
        Self { inner }
    }

    /// 標準 PathBuf を取得する。
    pub fn into_std(self) -> StdPathBuf {
        self.inner
    }

    /// 借用参照を取得する。
    pub fn as_path(&self) -> Path<'_> {
        Path { inner: &self.inner }
    }

    /// 標準 Path への参照を得る。
    pub fn as_std_path(&self) -> &StdPath {
        &self.inner
    }

    /// 正規化済みの PathBuf を返す。
    pub fn normalize(&self) -> PathBuf {
        PathBuf {
            inner: normalize_components(&self.inner),
        }
    }

    /// このパスが絶対パスかを返す。
    pub fn is_absolute(&self) -> bool {
        self.inner.is_absolute()
    }

    /// セグメントを結合して新しい PathBuf を返す。
    pub fn join(&self, segment: impl AsRef<str>) -> PathResult<PathBuf> {
        let segment_ref = segment.as_ref();
        validate_input(segment_ref)?;
        let mut joined = self.inner.clone();
        if !segment_ref.is_empty() {
            joined.push(segment_ref);
        }
        Ok(PathBuf {
            inner: normalize_components(&joined),
        })
    }

    /// 親ディレクトリを返す。
    pub fn parent(&self) -> Option<PathBuf> {
        self.inner.parent().map(|parent| PathBuf {
            inner: parent.to_path_buf(),
        })
    }

    /// パスを lossless に文字列へ変換する。
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.inner.to_string_lossy()
    }

    /// コンポーネント一覧を UTF-8 文字列で返す。
    pub fn components_as_strings(&self) -> Vec<String> {
        self.inner
            .components()
            .map(|component| match component {
                Component::CurDir => ".".to_string(),
                Component::ParentDir => "..".to_string(),
                _ => component.as_os_str().to_string_lossy().into_owned(),
            })
            .collect()
    }
}

impl<'a> Path<'a> {
    /// 標準 Path を参照で取得する。
    pub fn as_std(&self) -> &'a StdPath {
        self.inner
    }

    /// 絶対パスかを返す。
    pub fn is_absolute(&self) -> bool {
        self.inner.is_absolute()
    }

    /// 文字列表現を取得する。
    pub fn to_string_lossy(&self) -> Cow<'a, str> {
        self.inner.to_string_lossy()
    }
}

impl PathError {
    pub fn new(kind: PathErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            invalid_input: None,
            origin: PathErrorOrigin::Generic,
            effects: None,
        }
    }

    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.invalid_input = Some(input.into());
        self
    }

    pub fn kind(&self) -> PathErrorKind {
        self.kind
    }

    pub fn with_glob_pattern(mut self, pattern: impl Into<String>) -> Self {
        let ctx = match self.origin {
            PathErrorOrigin::Glob(ref mut existing) => {
                existing.pattern = Some(pattern.into());
                return self;
            }
            _ => GlobContext::new(pattern.into()),
        };
        self.origin = PathErrorOrigin::Glob(ctx);
        self
    }

    pub fn with_glob_offending_path(mut self, path: impl Into<String>) -> Self {
        match self.origin {
            PathErrorOrigin::Glob(ref mut ctx) => {
                ctx.offending_path = Some(path.into());
            }
            _ => {
                let mut ctx = GlobContext::new(String::new());
                ctx.offending_path = Some(path.into());
                self.origin = PathErrorOrigin::Glob(ctx);
            }
        }
        self
    }

    pub fn with_effects(mut self, effects: EffectLabels) -> Self {
        self.effects = Some(effects);
        self
    }
}

impl IntoDiagnostic for PathError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let PathError {
            kind,
            message,
            invalid_input,
            origin,
            effects,
        } = self;
        match origin {
            PathErrorOrigin::Glob(context) => {
                glob_diagnostic(kind, message, invalid_input, context, effects)
            }
            PathErrorOrigin::Generic => generic_path_diagnostic(kind, message),
        }
    }
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)?;
        if let Some(ref input) = self.invalid_input {
            write!(f, " ({input})")?;
        }
        Ok(())
    }
}

impl Error for PathError {}

impl<'a> TryFrom<Str<'a>> for PathBuf {
    type Error = PathError;

    fn try_from(value: Str<'a>) -> Result<Self, Self::Error> {
        PathBuf::try_from(value.as_str())
    }
}

impl TryFrom<&str> for PathBuf {
    type Error = PathError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        validate_input(value)?;
        Ok(PathBuf {
            inner: StdPathBuf::from(value),
        })
    }
}

impl From<PathBuf> for StdPathBuf {
    fn from(value: PathBuf) -> Self {
        value.into_std()
    }
}

/// `path(str)` に該当するエントリポイント。
pub fn path(text: Str<'_>) -> PathResult<PathBuf> {
    PathBuf::try_from(text)
}

/// `join(base, segment)` のヘルパ。
pub fn join(base: &PathBuf, segment: impl AsRef<str>) -> PathResult<PathBuf> {
    base.join(segment)
}

/// `normalize(path)` のヘルパ。
pub fn normalize(path: &PathBuf) -> PathBuf {
    path.normalize()
}

/// `parent(path)` のヘルパ。
pub fn parent(path: &PathBuf) -> Option<PathBuf> {
    path.parent()
}

/// `is_absolute(path)` のヘルパ。
pub fn is_absolute(path: &PathBuf) -> bool {
    path.is_absolute()
}

/// `components(path)` のヘルパ。
pub fn components(path: &PathBuf) -> Vec<String> {
    path.components_as_strings()
}

pub(super) fn validate_input(value: &str) -> PathResult<()> {
    if value.is_empty() {
        return Err(PathError::new(
            PathErrorKind::Empty,
            "path must not be empty",
        ));
    }
    if value.as_bytes().iter().any(|b| *b == 0) {
        return Err(
            PathError::new(PathErrorKind::NullByte, "path contains null byte").with_input(value),
        );
    }
    Ok(())
}

pub(super) fn normalize_components(path: &StdPath) -> StdPathBuf {
    let mut normalized = StdPathBuf::new();
    let is_absolute = path.is_absolute();
    for component in path.components() {
        match component {
            Component::CurDir => continue,
            Component::ParentDir => {
                if !normalized.pop() {
                    if !is_absolute {
                        normalized.push(component.as_os_str());
                    }
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }

    if normalized.as_os_str().is_empty() {
        if is_absolute {
            normalized.push(std::path::MAIN_SEPARATOR.to_string());
        } else {
            normalized.push(".");
        }
    }

    normalized
}

fn glob_diagnostic(
    kind: PathErrorKind,
    message: String,
    invalid_input: Option<String>,
    context: GlobContext,
    effects: Option<EffectLabels>,
) -> GuardDiagnostic {
    let mut glob_map = Map::new();
    if let Some(pattern) = context.pattern {
        glob_map.insert("pattern".into(), Value::String(pattern));
    }
    if let Some(offending) = context.offending_path {
        glob_map.insert("offending_path".into(), Value::String(offending));
    }
    if let Some(input) = invalid_input {
        glob_map.insert("input".into(), Value::String(input));
    }

    let mut extensions = Map::new();
    if !glob_map.is_empty() {
        let mut io_extensions = Map::new();
        io_extensions.insert("glob".into(), Value::Object(glob_map.clone()));
        extensions.insert("io".into(), Value::Object(io_extensions));
    }

    let mut effects_for_audit = None;
    if let Some(labels) = effects {
        let encoded = encode_effect_labels(labels);
        effects_for_audit = Some(encoded.clone());
        extensions.insert("effects".into(), Value::Object(encoded));
    }
    extensions.insert("message".into(), Value::String(message.clone()));

    let mut audit_metadata = Map::new();
    for (key, value) in glob_map {
        audit_metadata.insert(format!("io.glob.{key}"), value);
    }
    if let Some(effects_map) = effects_for_audit {
        for (key, value) in effects_map {
            audit_metadata.insert(format!("io.effects.{key}"), value);
        }
    }

    GuardDiagnostic {
        code: glob_diagnostic_code(kind),
        domain: "runtime",
        severity: DiagnosticSeverity::Error,
        message: format!("Core.Path glob failed: {message}"),
        notes: Vec::new(),
        extensions,
        audit_metadata,
    }
}

fn generic_path_diagnostic(kind: PathErrorKind, message: String) -> GuardDiagnostic {
    let formatted = format!("Core.Path {:?} error: {}", kind, message);
    let mut extensions = Map::new();
    extensions.insert("message".into(), Value::String(message));
    GuardDiagnostic {
        code: PATH_GENERIC_ERROR_CODE,
        domain: "runtime",
        severity: DiagnosticSeverity::Error,
        message: formatted,
        notes: Vec::new(),
        extensions,
        audit_metadata: Map::new(),
    }
}

fn glob_diagnostic_code(kind: PathErrorKind) -> &'static str {
    match kind {
        PathErrorKind::InvalidPattern => PATH_GLOB_INVALID_PATTERN_CODE,
        PathErrorKind::UnsupportedPlatform => PATH_GLOB_UNSUPPORTED_PLATFORM_CODE,
        PathErrorKind::Io => PATH_GLOB_IO_ERROR_CODE,
        PathErrorKind::Empty | PathErrorKind::NullByte | PathErrorKind::InvalidEncoding => {
            PATH_GLOB_INVALID_INPUT_CODE
        }
    }
}

pub(crate) fn encode_effect_labels(labels: EffectLabels) -> Map<String, Value> {
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

#[cfg(all(test, feature = "core-path"))]
mod tests {
    use super::{glob, validate_path, PathBuf, SecurityPolicy};
    use crate::prelude::ensure::{GuardDiagnostic, IntoDiagnostic};
    use crate::text::Str;
    use serde_json::Value;
    use std::path::PathBuf as StdPathBuf;

    fn boolean_extension(diag: &GuardDiagnostic, key: &str) -> Option<bool> {
        diag.extensions
            .get("effects")
            .and_then(Value::as_object)
            .and_then(|effects| effects.get(key))
            .and_then(Value::as_bool)
    }

    #[test]
    fn glob_invalid_pattern_records_io_effects() {
        let error = glob(Str::from("[")).expect_err("invalid pattern should produce a PathError");
        let diagnostic = error.into_diagnostic();
        assert_eq!(
            boolean_extension(&diagnostic, "io"),
            Some(true),
            "glob errors must mark io effect"
        );
        assert_eq!(
            boolean_extension(&diagnostic, "io_blocking"),
            Some(true),
            "glob errors must mark io_blocking effect"
        );
    }

    #[test]
    fn validate_path_violation_marks_security_effect() {
        let path = PathBuf::from_std(StdPathBuf::from("../etc/passwd"));
        let policy = SecurityPolicy::new();
        let error = validate_path(&path, &policy)
            .expect_err("relative path should violate the default policy");
        let diagnostic = error.into_diagnostic();
        assert_eq!(
            boolean_extension(&diagnostic, "security"),
            Some(true),
            "security policy diagnostics must set the security effect flag"
        );
    }
}
