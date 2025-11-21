#![allow(dead_code)]

//! Collector トレイトと監査メタデータの骨格実装。
//!
//! - 仕様出典: `docs/spec/3-1-core-prelude-iteration.md` §3.4
//! - WBS: 3.1b F1（Collector トレイト骨格 & EffectMarker）

mod list;
mod map;
mod set;
mod string;
mod table;
mod vec;

pub use list::{List, ListCollector};
pub use map::{Map, MapCollector};
pub use set::{Set, SetCollector};
pub use string::{StringCollector, StringError};
pub use table::{Table, TableCollector};
pub use vec::VecCollector;

use super::{
    ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic},
    iter::{EffectLabels, StageRequirement, StageRequirementDescriptor},
};
use serde_json::{Map as JsonObject, Number, Value};

/// `Collector::with_capacity` 用の EffectMarker。
pub const EFFECT_MARKER_WITH_CAPACITY: &str = "collector.effect.mem_reservation";
/// `Collector::reserve` 用の EffectMarker。
pub const EFFECT_MARKER_RESERVE: &str = "collector.effect.reserve";
/// `Collector::finish` 用の EffectMarker。
pub const EFFECT_MARKER_FINISH: &str = "collector.effect.finish";

const COLLECTOR_EXTENSION_KEY: &str = "prelude.collector";
const COLLECTOR_AUDIT_PREFIX: &str = "collector.";
const COLLECTOR_DIAGNOSTIC_CODE: &str = "core.prelude.collector_failed";
const COLLECTOR_DIAGNOSTIC_DOMAIN: &str = "runtime";

/// Collector 実装が返す監査済みの結果。
#[derive(Debug, Clone)]
pub struct CollectOutcome<C> {
    value: C,
    audit: CollectorAuditTrail,
}

impl<C> CollectOutcome<C> {
    /// 新しい結果を生成する。
    pub fn new(value: C, audit: CollectorAuditTrail) -> Self {
        Self { value, audit }
    }

    /// 結果の参照を返す。
    pub fn value(&self) -> &C {
        &self.value
    }

    /// 監査情報を返す。
    pub fn audit(&self) -> &CollectorAuditTrail {
        &self.audit
    }

    /// 監査情報とともに値を取り出す。
    pub fn into_parts(self) -> (C, CollectorAuditTrail) {
        (self.value, self.audit)
    }

    /// 値を写像する（監査情報は保持）。
    pub fn map<U>(self, f: impl FnOnce(C) -> U) -> CollectOutcome<U> {
        let CollectOutcome { value, audit } = self;
        CollectOutcome {
            value: f(value),
            audit,
        }
    }
}

/// Collector が保持するステージ情報。
#[derive(Debug, Clone)]
pub struct CollectorStageProfile {
    requirement: StageRequirement,
    actual: &'static str,
    capability: Option<&'static str>,
    kind: CollectorKind,
}

impl CollectorStageProfile {
    /// Collector 種別に応じた既定プロファイルを生成する。
    pub fn for_kind(kind: CollectorKind) -> Self {
        let requirement = kind.default_requirement();
        let actual = kind.default_actual();
        let capability = kind.capability_id();
        Self {
            requirement,
            actual,
            capability,
            kind,
        }
    }

    /// 監査ログ向け Snapshot を作る。
    pub fn snapshot(&self, source: impl Into<String>) -> CollectorStageSnapshot {
        CollectorStageSnapshot {
            required: self.requirement.descriptor(),
            actual: self.actual,
            capability: self.capability,
            kind: self.kind.as_str(),
            source: source.into(),
        }
    }

    /// Stage 要件を取得する。
    pub fn requirement(&self) -> StageRequirement {
        self.requirement
    }

    /// Collector 種別を取得する。
    pub fn kind(&self) -> CollectorKind {
        self.kind
    }
}

/// 監査 Snapshot。
#[derive(Debug, Clone)]
pub struct CollectorStageSnapshot {
    pub required: StageRequirementDescriptor,
    pub actual: &'static str,
    pub capability: Option<&'static str>,
    pub kind: &'static str,
    pub source: String,
}

impl CollectorStageSnapshot {
    fn stage_mismatch(&self) -> bool {
        match self.required.mode {
            "exact" => self.actual != self.required.stage,
            "at_least" => stage_rank(self.actual) < stage_rank(self.required.stage),
            _ => false,
        }
    }
}

fn stage_rank(stage: &str) -> u8 {
    match stage {
        "stable" => 3,
        "beta" => 2,
        "alpha" => 1,
        "experimental" => 0,
        _ => 0,
    }
}

/// Collector の効果記録。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CollectorEffectMarkers {
    pub mem_reservation: usize,
    pub reserve: usize,
    pub finish: usize,
}

impl CollectorEffectMarkers {
    /// `with_capacity` / `reserve` で追加したバイト数を記録する。
    pub fn record_mem_reservation(&mut self, amount: usize) {
        self.mem_reservation = self.mem_reservation.saturating_add(amount);
    }

    /// `reserve` で拡張したバイト数を記録する。
    pub fn record_reserve(&mut self, amount: usize) {
        self.reserve = self.reserve.saturating_add(amount);
    }

    /// `finish` の呼び出しを記録する。
    pub fn record_finish(&mut self) {
        self.finish = self.finish.saturating_add(1);
    }
}

/// Collector 監査で保持する共通情報。
#[derive(Debug, Clone)]
pub struct CollectorAuditTrail {
    pub kind: CollectorKind,
    pub stage: CollectorStageSnapshot,
    pub effects: EffectLabels,
    pub markers: CollectorEffectMarkers,
}

impl CollectorAuditTrail {
    /// 新しい監査情報を生成する。
    pub fn new(
        kind: CollectorKind,
        stage: CollectorStageSnapshot,
        effects: EffectLabels,
        markers: CollectorEffectMarkers,
    ) -> Self {
        Self {
            kind,
            stage,
            effects,
            markers,
        }
    }

    fn extension_payload(&self) -> JsonObject<String, Value> {
        let mut obj = JsonObject::new();
        obj.insert("kind".into(), Value::String(self.kind.as_str().into()));
        obj.insert(
            "stage_required".into(),
            Value::String(self.stage.required.stage.into()),
        );
        obj.insert(
            "stage_mode".into(),
            Value::String(self.stage.required.mode.into()),
        );
        obj.insert(
            "stage_actual".into(),
            Value::String(self.stage.actual.into()),
        );
        obj.insert(
            "stage_mismatch".into(),
            Value::Bool(self.stage.stage_mismatch()),
        );
        obj.insert(
            "capability".into(),
            self.stage
                .capability
                .map(|cap| Value::String(cap.into()))
                .unwrap_or(Value::Null),
        );
        obj.insert("source".into(), Value::String(self.stage.source.clone()));

        let mut effects = JsonObject::new();
        effects.insert("mem".into(), Value::Bool(self.effects.mem));
        effects.insert("mut".into(), Value::Bool(self.effects.mutating));
        effects.insert("debug".into(), Value::Bool(self.effects.debug));
        effects.insert(
            "async_pending".into(),
            Value::Bool(self.effects.async_pending),
        );
        effects.insert(
            "predicate_calls".into(),
            Value::Number(Number::from(self.effects.predicate_calls as u64)),
        );
        effects.insert(
            "mem_bytes".into(),
            Value::Number(Number::from(self.effects.mem_bytes as u64)),
        );
        obj.insert("effects".into(), Value::Object(effects));

        let mut markers = JsonObject::new();
        markers.insert(
            "mem_reservation".into(),
            Value::Number(Number::from(self.markers.mem_reservation as u64)),
        );
        markers.insert(
            "reserve".into(),
            Value::Number(Number::from(self.markers.reserve as u64)),
        );
        markers.insert(
            "finish".into(),
            Value::Number(Number::from(self.markers.finish as u64)),
        );
        obj.insert("markers".into(), Value::Object(markers));
        obj
    }

    fn audit_metadata(&self) -> JsonObject<String, Value> {
        let mut metadata = JsonObject::new();
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}kind"),
            Value::String(self.kind.as_str().into()),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}stage.required"),
            Value::String(self.stage.required.stage.into()),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}stage.mode"),
            Value::String(self.stage.required.mode.into()),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}stage.actual"),
            Value::String(self.stage.actual.into()),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}stage.mismatch"),
            Value::Bool(self.stage.stage_mismatch()),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}stage.source"),
            Value::String(self.stage.source.clone()),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}capability"),
            self.stage
                .capability
                .map(|cap| Value::String(cap.into()))
                .unwrap_or(Value::Null),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.mem"),
            Value::Bool(self.effects.mem),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.mut"),
            Value::Bool(self.effects.mutating),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.debug"),
            Value::Bool(self.effects.debug),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.async_pending"),
            Value::Bool(self.effects.async_pending),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.predicate_calls"),
            Value::Number(Number::from(self.effects.predicate_calls as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.mem_reservation"),
            Value::Number(Number::from(self.markers.mem_reservation as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.reserve"),
            Value::Number(Number::from(self.markers.reserve as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.finish"),
            Value::Number(Number::from(self.markers.finish as u64)),
        );
        metadata
    }
}

/// Collector 種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorKind {
    List,
    Vec,
    Map,
    Set,
    String,
    Table,
    Histogram,
    Custom(&'static str),
}

impl CollectorKind {
    /// Stage 要件を取得する。
    pub fn default_requirement(&self) -> StageRequirement {
        match self {
            CollectorKind::List | CollectorKind::Set => StageRequirement::Exact("stable"),
            CollectorKind::Custom(_) => StageRequirement::AtLeast("beta"),
            _ => StageRequirement::AtLeast("beta"),
        }
    }

    /// 実際の Stage（監査用ラベル）。
    pub fn default_actual(&self) -> &'static str {
        match self {
            CollectorKind::List | CollectorKind::Set => "stable",
            CollectorKind::Histogram => "experimental",
            CollectorKind::Custom(_) => "unknown",
            _ => "beta",
        }
    }

    /// Capability ID を返す（存在する場合）。
    pub fn capability_id(&self) -> Option<&'static str> {
        match self {
            CollectorKind::List => Some("core.collector.list"),
            CollectorKind::Vec => Some("core.collector.vec"),
            CollectorKind::Map => Some("core.collector.map"),
            CollectorKind::Set => Some("core.collector.set"),
            CollectorKind::String => Some("core.collector.string"),
            CollectorKind::Table => Some("core.collector.table"),
            CollectorKind::Histogram => Some("core.collector.histogram"),
            CollectorKind::Custom(_) => None,
        }
    }

    /// ラベルを取得する。
    pub fn as_str(&self) -> &'static str {
        match self {
            CollectorKind::List => "list",
            CollectorKind::Vec => "vec",
            CollectorKind::Map => "map",
            CollectorKind::Set => "set",
            CollectorKind::String => "string",
            CollectorKind::Table => "table",
            CollectorKind::Histogram => "histogram",
            CollectorKind::Custom(_) => "custom",
        }
    }

    /// 表示用名称。
    pub fn display_name(&self) -> &'static str {
        match self {
            CollectorKind::List => "ListCollector",
            CollectorKind::Vec => "VecCollector",
            CollectorKind::Map => "MapCollector",
            CollectorKind::Set => "SetCollector",
            CollectorKind::String => "StringCollector",
            CollectorKind::Table => "TableCollector",
            CollectorKind::Histogram => "HistogramCollector",
            CollectorKind::Custom(name) => name,
        }
    }
}

/// Collector の実装契約。
pub trait Collector<T, C> {
    /// 収集エラー。
    type Error: IntoDiagnostic;

    /// `@pure` 初期化。
    fn new() -> Self
    where
        Self: Sized;

    /// `effect {mem}`: 事前確保（`EffectMarker` = [`EFFECT_MARKER_WITH_CAPACITY`]).
    fn with_capacity(capacity: usize) -> Self
    where
        Self: Sized,
    {
        let _ = capacity;
        Self::new()
    }

    /// `effect {mut}`: 値を押し込む。
    fn push(&mut self, value: T) -> Result<(), Self::Error>;

    /// `effect {mut, mem}`: 追加確保（`EffectMarker` = [`EFFECT_MARKER_RESERVE`]).
    fn reserve(&mut self, additional: usize) -> Result<(), Self::Error> {
        let _ = additional;
        Ok(())
    }

    /// `effect {mem}`: 終端処理（`EffectMarker` = [`EFFECT_MARKER_FINISH`]).
    fn finish(self) -> C
    where
        Self: Sized;

    /// `@pure`: 所有権をそのまま返す（`finish` の軽量版）。
    fn into_inner(self) -> C
    where
        Self: Sized,
    {
        self.finish()
    }
}

/// Collector エラー種別。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectErrorKind {
    MemoryError,
    CapacityOverflow,
    DuplicateKey,
    InvalidEncoding,
    Custom(&'static str),
}

impl CollectErrorKind {
    /// ラベルを返す。
    pub fn as_str(&self) -> &'static str {
        match self {
            CollectErrorKind::MemoryError => "memory_error",
            CollectErrorKind::CapacityOverflow => "capacity_overflow",
            CollectErrorKind::DuplicateKey => "duplicate_key",
            CollectErrorKind::InvalidEncoding => "invalid_encoding",
            CollectErrorKind::Custom(label) => label,
        }
    }
}

/// Collector が返す標準エラー。
#[derive(Debug, Clone)]
pub struct CollectError {
    kind: CollectErrorKind,
    message: String,
    detail: Option<String>,
    audit: CollectorAuditTrail,
    error_key: Option<String>,
}

impl CollectError {
    /// 新しいエラーを構築する。
    pub fn new(
        kind: CollectErrorKind,
        message: impl Into<String>,
        audit: CollectorAuditTrail,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            detail: None,
            audit,
            error_key: None,
        }
    }

    /// 追加情報を付与する。
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// エラーキー（例: 重複キー）を追加する。
    pub fn with_error_key(mut self, key: impl Into<String>) -> Self {
        self.error_key = Some(key.into());
        self
    }

    /// エラー種別を取得する。
    pub fn kind(&self) -> &CollectErrorKind {
        &self.kind
    }

    /// 監査情報を取得する。
    pub fn audit(&self) -> &CollectorAuditTrail {
        &self.audit
    }
}

impl IntoDiagnostic for CollectError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let CollectError {
            kind,
            message,
            detail,
            audit,
            error_key,
        } = self;

        let mut extensions = audit.extension_payload();
        if let Some(key) = error_key.as_ref() {
            extensions.insert("error_key".into(), Value::String(key.clone()));
        }
        extensions.insert("error_kind".into(), Value::String(kind.as_str().into()));
        extensions.insert("message".into(), Value::String(message.clone()));
        if let Some(detail) = detail.clone() {
            extensions.insert("detail".into(), Value::String(detail));
        }

        let mut metadata = audit.audit_metadata();
        if let Some(key) = error_key {
            metadata.insert(
                format!("{COLLECTOR_AUDIT_PREFIX}error.key"),
                Value::String(key),
            );
        }

        GuardDiagnostic {
            code: COLLECTOR_DIAGNOSTIC_CODE,
            domain: COLLECTOR_DIAGNOSTIC_DOMAIN,
            severity: DiagnosticSeverity::Error,
            message: format!("{} failed: {}", audit.kind.display_name(), message),
            extensions: {
                let mut root = JsonObject::new();
                root.insert(COLLECTOR_EXTENSION_KEY.into(), Value::Object(extensions));
                root
            },
            audit_metadata: metadata,
        }
    }
}
