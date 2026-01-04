use serde_json::{json, Map, Value};
use std::{option::Option as RemlOption, result::Result as RemlResult};

type StdOption<T> = std::option::Option<T>;

const GUARD_EXTENSION_KEY: &str = "prelude.guard";
const GUARD_AUDIT_PREFIX: &str = "core.prelude.guard.";
const ENSURE_DIAGNOSTIC_CODE: &str = "core.prelude.ensure_failed";
const RUNTIME_DOMAIN: &str = "runtime";

/// Guard の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreludeGuardKind {
    /// `ensure` による真偽値チェック。
    Ensure,
    /// `ensure_not_null` による存在チェック。
    EnsureNotNull,
}

impl PreludeGuardKind {
    fn as_str(&self) -> &'static str {
        match self {
            PreludeGuardKind::Ensure => "ensure",
            PreludeGuardKind::EnsureNotNull => "ensure_not_null",
        }
    }
}

/// Guard 失敗時に記録するメタデータ。
#[derive(Debug, Clone)]
pub struct PreludeGuardMetadata {
    kind: PreludeGuardKind,
    trigger: String,
    pointer_class: StdOption<String>,
    stage: StdOption<String>,
    module_path: StdOption<String>,
}

impl PreludeGuardMetadata {
    /// Guard メタデータを生成する。
    pub fn new(kind: PreludeGuardKind, trigger: impl Into<String>) -> Self {
        Self {
            kind,
            trigger: trigger.into(),
            pointer_class: StdOption::None,
            stage: StdOption::None,
            module_path: StdOption::None,
        }
    }

    /// Guard を発火させた呼び出し元モジュール名を設定する。
    pub fn with_module_path(mut self, module_path: impl Into<String>) -> Self {
        self.module_path = StdOption::Some(module_path.into());
        self
    }

    /// Guard が扱うポインタの種類を設定する（例: `ffi` / `plugin` / `core`）。
    pub fn with_pointer_class(mut self, class: impl Into<String>) -> Self {
        self.pointer_class = StdOption::Some(class.into());
        self
    }

    /// Stage 要件を記録する。
    pub fn with_stage(mut self, stage: impl Into<String>) -> Self {
        self.stage = StdOption::Some(stage.into());
        self
    }

    fn extension_payload(&self) -> Value {
        let mut obj = Map::new();
        obj.insert("kind".into(), Value::String(self.kind.as_str().into()));
        obj.insert("trigger".into(), Value::String(self.trigger.clone()));
        obj.insert(
            "pointer_class".into(),
            self.pointer_class
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "stage".into(),
            self.stage.clone().map(Value::String).unwrap_or(Value::Null),
        );
        obj.insert(
            "module".into(),
            self.module_path
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
        );
        Value::Object(obj)
    }

    fn audit_metadata(&self) -> Map<String, Value> {
        let mut metadata = Map::new();
        metadata.insert(
            format!("{GUARD_AUDIT_PREFIX}kind"),
            Value::String(self.kind.as_str().into()),
        );
        metadata.insert(
            format!("{GUARD_AUDIT_PREFIX}trigger"),
            Value::String(self.trigger.clone()),
        );
        metadata.insert(
            format!("{GUARD_AUDIT_PREFIX}pointer_class"),
            self.pointer_class
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
        );
        metadata.insert(
            format!("{GUARD_AUDIT_PREFIX}stage"),
            self.stage.clone().map(Value::String).unwrap_or(Value::Null),
        );
        metadata.insert(
            format!("{GUARD_AUDIT_PREFIX}module"),
            self.module_path
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
        );
        metadata
    }
}

/// 診断の Severity。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl DiagnosticSeverity {
    fn as_str(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Hint => "hint",
        }
    }
}

/// Guard 失敗を表現する診断。
#[derive(Debug, Clone)]
pub struct GuardDiagnostic {
    pub code: &'static str,
    pub domain: &'static str,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub notes: Vec<DiagnosticNote>,
    pub extensions: Map<String, Value>,
    pub audit_metadata: Map<String, Value>,
}

/// 診断の補足情報（notes）。
#[derive(Debug, Clone)]
pub struct DiagnosticNote {
    pub message: String,
}

impl DiagnosticNote {
    pub fn plain(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl GuardDiagnostic {
    /// JSON へ変換する補助（テスト・ロギング向け）。
    pub fn into_json(self) -> Value {
        let mut root = Map::new();
        root.insert("code".into(), Value::String(self.code.into()));
        root.insert("domain".into(), Value::String(self.domain.into()));
        root.insert(
            "severity".into(),
            Value::String(self.severity.as_str().into()),
        );
        root.insert("message".into(), Value::String(self.message));
        root.insert(
            "notes".into(),
            Value::Array(
                self.notes
                    .into_iter()
                    .map(|note| json!({ "message": note.message }))
                    .collect(),
            ),
        );
        root.insert("extensions".into(), Value::Object(self.extensions));
        root.insert("audit".into(), Value::Object(self.audit_metadata));
        Value::Object(root)
    }
}

/// 診断への変換トレイト。
pub trait IntoDiagnostic {
    fn into_diagnostic(self) -> GuardDiagnostic;
}

impl IntoDiagnostic for GuardDiagnostic {
    fn into_diagnostic(self) -> GuardDiagnostic {
        self
    }
}

/// Guard 用エラー。
#[derive(Debug, Clone)]
pub struct EnsureError {
    message: String,
    metadata: PreludeGuardMetadata,
}

impl EnsureError {
    /// 新しい Guard エラーを生成する。
    pub fn new(message: impl Into<String>, metadata: PreludeGuardMetadata) -> Self {
        Self {
            message: message.into(),
            metadata,
        }
    }

    /// メタデータを取得する。
    pub fn metadata(&self) -> &PreludeGuardMetadata {
        &self.metadata
    }

    /// メッセージを取得する。
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl IntoDiagnostic for EnsureError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let mut extensions = Map::new();
        extensions.insert(
            GUARD_EXTENSION_KEY.into(),
            self.metadata.extension_payload(),
        );

        GuardDiagnostic {
            code: ENSURE_DIAGNOSTIC_CODE,
            domain: RUNTIME_DOMAIN,
            severity: DiagnosticSeverity::Error,
            message: self.message,
            notes: Vec::new(),
            extensions,
            audit_metadata: self.metadata.audit_metadata(),
        }
    }
}

/// `ensure` の失敗要因を構築するビルダー。
#[derive(Debug, Clone)]
pub struct EnsureErrorBuilder {
    metadata: PreludeGuardMetadata,
    message: StdOption<String>,
}

impl EnsureErrorBuilder {
    /// Guard kind とトリガーを指定してビルダーを開始する。
    pub fn new(kind: PreludeGuardKind, trigger: impl Into<String>) -> Self {
        Self {
            metadata: PreludeGuardMetadata::new(kind, trigger),
            message: StdOption::None,
        }
    }

    /// エラーメッセージを上書きする。
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = StdOption::Some(message.into());
        self
    }

    /// モジュール情報を設定する。
    pub fn module_path(mut self, module: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_module_path(module);
        self
    }

    /// ポインタ分類を設定する。
    pub fn pointer_class(mut self, class: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_pointer_class(class);
        self
    }

    /// Stage 情報を設定する。
    pub fn stage(mut self, stage: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_stage(stage);
        self
    }

    /// `EnsureError` を構築する。
    pub fn build(self) -> EnsureError {
        let default = format!(
            "{} guard `{}` failed",
            self.metadata.kind.as_str(),
            self.metadata.trigger
        );
        EnsureError::new(self.message.unwrap_or(default), self.metadata)
    }
}

/// `ensure` ヘルパ。
#[inline]
pub fn ensure<E>(condition: bool, err: impl FnOnce() -> E) -> RemlResult<(), E> {
    if condition {
        RemlResult::Ok(())
    } else {
        RemlResult::Err(err())
    }
}

/// `ensure_not_null` ヘルパ。
#[inline]
pub fn ensure_not_null<T, E>(value: RemlOption<T>, err: impl FnOnce() -> E) -> RemlResult<T, E> {
    match value {
        RemlOption::Some(value) => RemlResult::Ok(value),
        RemlOption::None => RemlResult::Err(err()),
    }
}
