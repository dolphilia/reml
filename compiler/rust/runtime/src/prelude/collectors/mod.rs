#![allow(dead_code)]

//! Collector トレイトと監査メタデータの骨格実装。
//!
//! - 仕様出典: `docs/spec/3-1-core-prelude-iteration.md` §3.4
//! - WBS: 3.1b F1（Collector トレイト骨格 & EffectMarker）

mod list;
mod map;
mod numeric;
mod set;
mod string;
mod table;
mod vec;

pub use list::{List, ListCollector};
pub use map::{Map, MapCollector};
pub use numeric::NumericCollector;
pub use set::{Set, SetCollector};
pub use string::{StringCollector, StringError};
pub use table::{Table, TableCollector};
pub use vec::VecCollector;

use super::{
    ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic},
    iter::{
        EffectLabels, IterError, StageRequirement as IteratorStageRequirement,
        StageRequirementDescriptor,
    },
};
use crate::{
    capability::registry::{CapabilityError, CapabilityRegistry},
    collections::audit_bridge::ChangeSet,
    StageId, StageRequirement as RegistryStageRequirement,
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
const CORE_COLLECTIONS_AUDIT_CAPABILITY: &str = "core.collections.audit";
const CORE_COLLECTIONS_AUDIT_EFFECTS: [&str; 2] = ["audit", "mem"];

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

    /// 監査情報への可変参照を取得する。
    pub fn audit_mut(&mut self) -> &mut CollectorAuditTrail {
        &mut self.audit
    }

    /// ChangeSet を記録し effect 情報を更新する。
    pub fn record_change_set(mut self, change_set: &ChangeSet) -> Self {
        self.audit.record_change_set(change_set);
        self
    }

    pub fn ensure_audit_capability(self) -> Result<Self, CollectError> {
        if self.audit.effects.audit {
            ensure_core_collections_audit(&self.audit)?;
        }
        Ok(self)
    }
}

/// Collector が保持するステージ情報。
#[derive(Debug, Clone)]
pub struct CollectorStageProfile {
    requirement: IteratorStageRequirement,
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
    pub fn requirement(&self) -> IteratorStageRequirement {
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
    pub cell_mutations: usize,
    pub time_calls: usize,
    pub io_blocking_ops: usize,
    pub io_async_ops: usize,
    pub security_checks: usize,
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

    /// `Cell` の内部可変性操作を記録する。
    pub fn record_cell_op(&mut self) {
        self.cell_mutations = self.cell_mutations.saturating_add(1);
    }

    /// 時刻取得系 API の呼び出し回数を記録する。
    pub fn record_time_call(&mut self, calls: usize) {
        if calls == 0 {
            return;
        }
        self.time_calls = self.time_calls.saturating_add(calls);
    }

    /// `effect {io.blocking}` を発生させた回数を記録する。
    pub fn record_io_blocking(&mut self) {
        self.io_blocking_ops = self.io_blocking_ops.saturating_add(1);
    }

    /// `effect {io.async}` を発生させた回数を記録する。
    pub fn record_io_async(&mut self) {
        self.io_async_ops = self.io_async_ops.saturating_add(1);
    }

    /// `effect {security}` を発生させた回数を記録する。
    pub fn record_security_check(&mut self) {
        self.security_checks = self.security_checks.saturating_add(1);
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
        effects.insert("audit".into(), Value::Bool(self.effects.audit));
        effects.insert("cell".into(), Value::Bool(self.effects.cell));
        effects.insert("rc".into(), Value::Bool(self.effects.rc));
        effects.insert(
            "rc_ops".into(),
            Value::Number(Number::from(self.effects.rc_ops as u64)),
        );
        effects.insert("unicode".into(), Value::Bool(self.effects.unicode));
        effects.insert("transfer".into(), Value::Bool(self.effects.transfer));
        effects.insert("time".into(), Value::Bool(self.effects.time));
        effects.insert(
            "predicate_calls".into(),
            Value::Number(Number::from(self.effects.predicate_calls as u64)),
        );
        effects.insert(
            "mem_bytes".into(),
            Value::Number(Number::from(self.effects.mem_bytes as u64)),
        );
        effects.insert("io".into(), Value::Bool(self.effects.io));
        effects.insert("io_blocking".into(), Value::Bool(self.effects.io_blocking));
        effects.insert("io_async".into(), Value::Bool(self.effects.io_async));
        effects.insert("security".into(), Value::Bool(self.effects.security));
        effects.insert(
            "time_calls".into(),
            Value::Number(Number::from(self.effects.time_calls as u64)),
        );
        effects.insert(
            "io_blocking_calls".into(),
            Value::Number(Number::from(self.effects.io_blocking_calls as u64)),
        );
        effects.insert(
            "io_async_calls".into(),
            Value::Number(Number::from(self.effects.io_async_calls as u64)),
        );
        effects.insert(
            "security_events".into(),
            Value::Number(Number::from(self.effects.security_events as u64)),
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
        markers.insert(
            "time_calls".into(),
            Value::Number(Number::from(self.effects.time_calls as u64)),
        );
        markers.insert(
            "io_blocking_ops".into(),
            Value::Number(Number::from(self.markers.io_blocking_ops as u64)),
        );
        markers.insert(
            "io_async_ops".into(),
            Value::Number(Number::from(self.markers.io_async_ops as u64)),
        );
        markers.insert(
            "security_checks".into(),
            Value::Number(Number::from(self.markers.security_checks as u64)),
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
            format!("{COLLECTOR_AUDIT_PREFIX}effect.audit"),
            Value::Bool(self.effects.audit),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.cell"),
            Value::Bool(self.effects.cell),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.rc"),
            Value::Bool(self.effects.rc),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.rc_ops"),
            Value::Number(Number::from(self.effects.rc_ops as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.transfer"),
            Value::Bool(self.effects.transfer),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.predicate_calls"),
            Value::Number(Number::from(self.effects.predicate_calls as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.mem_bytes"),
            Value::Number(Number::from(self.effects.mem_bytes as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.io"),
            Value::Bool(self.effects.io),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.io_blocking"),
            Value::Bool(self.effects.io_blocking),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.io_async"),
            Value::Bool(self.effects.io_async),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.security"),
            Value::Bool(self.effects.security),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.time"),
            Value::Bool(self.effects.time),
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
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.cell_mutations"),
            Value::Number(Number::from(self.markers.cell_mutations as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.time_calls"),
            Value::Number(Number::from(self.effects.time_calls as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.io_blocking_calls"),
            Value::Number(Number::from(self.effects.io_blocking_calls as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.io_async_calls"),
            Value::Number(Number::from(self.effects.io_async_calls as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.security_events"),
            Value::Number(Number::from(self.effects.security_events as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.io_blocking_ops"),
            Value::Number(Number::from(self.markers.io_blocking_ops as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.io_async_ops"),
            Value::Number(Number::from(self.markers.io_async_ops as u64)),
        );
        metadata.insert(
            format!("{COLLECTOR_AUDIT_PREFIX}effect.security_checks"),
            Value::Number(Number::from(self.markers.security_checks as u64)),
        );
        append_text_metadata(&mut metadata);
        metadata
    }

    /// 差分結果を effect メタデータへ反映する。
    pub fn record_change_set(&mut self, change_set: &ChangeSet) {
        let total = change_set.summary().total();
        if total > 0 {
            self.effects.audit = true;
        }
        self.effects.mem_bytes = self.effects.mem_bytes.saturating_add(total);
    }
}

fn ensure_core_collections_audit(audit: &CollectorAuditTrail) -> Result<(), CollectError> {
    let required_effects: Vec<String> = CORE_COLLECTIONS_AUDIT_EFFECTS
        .iter()
        .map(|value| value.to_string())
        .collect();
    CapabilityRegistry::registry()
        .verify_capability_stage(
            CORE_COLLECTIONS_AUDIT_CAPABILITY,
            RegistryStageRequirement::Exact(StageId::Stable),
            &required_effects,
        )
        .map(|_| ())
        .map_err(|err| {
            CollectError::capability_denied(CORE_COLLECTIONS_AUDIT_CAPABILITY, audit.clone(), err)
        })
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
    Numeric,
    Custom(&'static str),
}

impl CollectorKind {
    /// Stage 要件を取得する。
    pub fn default_requirement(&self) -> IteratorStageRequirement {
        match self {
            CollectorKind::List | CollectorKind::Set => IteratorStageRequirement::Exact("stable"),
            CollectorKind::Numeric => IteratorStageRequirement::AtLeast("beta"),
            CollectorKind::Custom(_) => IteratorStageRequirement::AtLeast("beta"),
            _ => IteratorStageRequirement::AtLeast("beta"),
        }
    }

    /// 実際の Stage（監査用ラベル）。
    pub fn default_actual(&self) -> &'static str {
        match self {
            CollectorKind::List | CollectorKind::Set => "stable",
            CollectorKind::Histogram => "experimental",
            CollectorKind::Numeric => "beta",
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
            CollectorKind::Numeric => Some("core.numeric.collector"),
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
            CollectorKind::Numeric => "numeric",
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
            CollectorKind::Numeric => "NumericCollector",
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

    /// `Iter` 側のエラーを Collector 側のエラー型へ写像する。
    fn iter_error(self, error: IterError) -> Self::Error
    where
        Self: Sized;
}

/// Collector エラー種別。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectErrorKind {
    MemoryError,
    CapacityOverflow,
    DuplicateKey,
    InvalidEncoding,
    IteratorFailure,
    UnstableOrder,
    CapabilityDenied,
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
            CollectErrorKind::IteratorFailure => "iterator_failure",
            CollectErrorKind::UnstableOrder => "unstable_order",
            CollectErrorKind::CapabilityDenied => "capability_denied",
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
    capability: Option<String>,
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
            capability: None,
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

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capability = Some(capability.into());
        self
    }

    pub fn capability_denied(
        capability: impl Into<String>,
        audit: CollectorAuditTrail,
        source: CapabilityError,
    ) -> Self {
        let capability = capability.into();
        CollectError {
            kind: CollectErrorKind::CapabilityDenied,
            message: format!("Capability '{capability}' denied: {source}"),
            detail: Some(source.to_string()),
            audit,
            error_key: None,
            capability: Some(capability),
        }
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
            capability,
        } = self;

        let mut extensions = audit.extension_payload();
        if let Some(key) = error_key.as_ref() {
            extensions.insert("error_key".into(), Value::String(key.clone()));
        }
        if let Some(cap) = capability.as_ref() {
            extensions.insert("collector.capability".into(), Value::String(cap.clone()));
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
        if let Some(cap) = capability {
            metadata.insert(
                format!("{COLLECTOR_AUDIT_PREFIX}capability"),
                Value::String(cap),
            );
        }

        GuardDiagnostic {
            code: COLLECTOR_DIAGNOSTIC_CODE,
            domain: COLLECTOR_DIAGNOSTIC_DOMAIN,
            severity: DiagnosticSeverity::Error,
            message: format!("{} failed: {}", audit.kind.display_name(), message),
            notes: Vec::new(),
            extensions: {
                let mut root = JsonObject::new();
                root.insert(COLLECTOR_EXTENSION_KEY.into(), Value::Object(extensions));
                root
            },
            audit_metadata: metadata,
        }
    }
}

fn append_text_metadata(metadata: &mut JsonObject<String, Value>) {
    if let Some(extra) = crate::text::take_text_audit_metadata() {
        for (key, value) in extra {
            metadata.insert(key, value);
        }
    }
}
