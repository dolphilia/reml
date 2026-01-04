#![allow(dead_code)]

//! Iter/Collector 実装の基盤となるコア構造体群。
//!
//! - `Iter<T>` は `Arc<IterState<T>>` を介して共有される遅延列。
//! - `IterState` はソース（`IterSource`）と効果タグ（`EffectSet`）、
//!   さらに Stage/Capability 情報（`IteratorStageProfile`）を保持する。
//! - `IterStep` は `Ready`/`Pending`/`Finished` の 3 状態を提供し、
//!   効果タグの計測には `EffectSet` を利用する。

use std::{
    borrow::Cow,
    collections::VecDeque,
    fmt,
    iter::FromIterator,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use super::collectors::{
    CollectError, CollectOutcome, Collector, CollectorAuditTrail, List, ListCollector, Map,
    MapCollector, Set, SetCollector, Table, TableCollector, VecCollector,
};
#[cfg(feature = "core_numeric")]
use super::collectors::{
    CollectorEffectMarkers, CollectorKind, CollectorStageProfile, NumericCollector,
};
use crate::collections::mutable::CoreVec;
#[cfg(feature = "core_numeric")]
use crate::diagnostics::{
    metric_required_effects, MetricsStageGuard, METRIC_CAPABILITY_ID, METRIC_STAGE_REQUIREMENT,
};
use serde::Serialize;

mod generators;
pub use generators::*;
mod adapters;
mod try_collect;

use try_collect::CollectorBridge;

/// 遅延列 `Iter<T>` の共有ハンドル。
#[derive(Debug)]
#[must_use = "Iter は遅延列のため、終端操作を呼び出して結果を利用してください"]
pub struct Iter<T> {
    core: Arc<IterCore<T>>,
}

/// `IntoIterator` 経由で `Iter` を走査するためのアダプタ。
#[derive(Clone, Debug)]
pub struct IterIntoIterator<T> {
    iter: Iter<T>,
}

#[derive(Debug)]
struct IterCore<T> {
    state: Mutex<IterState<T>>,
}

impl<T> Clone for Iter<T> {
    fn clone(&self) -> Self {
        Self {
            core: Arc::clone(&self.core),
        }
    }
}

impl<T> Iter<T> {
    /// `IterState` を共有ハンドルへ包む。
    pub fn from_state(state: IterState<T>) -> Self {
        Self {
            core: Arc::new(IterCore {
                state: Mutex::new(state),
            }),
        }
    }

    pub(crate) fn from_seed(mut seed: IterSeed<T>) -> Self {
        let stage_profile = seed.stage_profile().clone();
        let effects = seed.effects();
        let driver = seed.take_driver();
        let state = IterState::new(IterSource::Seed(seed), stage_profile)
            .with_effects(effects)
            .with_driver(driver);
        Self::from_state(state)
    }

    pub(crate) fn with_source(
        source: IterSource<T>,
        stage_profile: IteratorStageProfile,
        effects: EffectSet,
        driver: IterDriver<T>,
    ) -> Self {
        let state = IterState::new(source, stage_profile)
            .with_effects(effects)
            .with_driver(driver);
        Self::from_state(state)
    }

    /// 空の `Iter` を生成する。
    pub fn empty() -> Self {
        let stage_profile = IteratorStageProfile::for_kind(IteratorKind::CoreIter);
        Self::with_source(
            IterSource::Empty,
            stage_profile,
            EffectSet::PURE,
            IterDriver::Empty,
        )
    }

    /// Stage/Capability 情報のスナップショットを生成する。
    pub fn stage_snapshot(&self, source_name: impl Into<String>) -> IteratorStageSnapshot {
        let guard = self
            .core
            .state
            .lock()
            .expect("IterState poisoned during snapshot");
        guard.stage_profile.snapshot(source_name.into())
    }

    /// 効果ラベル（`iterator.effect.*`）を取得する。
    pub fn effect_labels(&self) -> EffectLabels {
        let guard = self
            .core
            .state
            .lock()
            .expect("IterState poisoned during effect_labels()");
        guard.effects.to_labels()
    }

    fn metadata_for_adapter(&self) -> (IteratorStageProfile, EffectSet) {
        let guard = self
            .core
            .state
            .lock()
            .expect("IterState poisoned during adapter metadata snapshot");
        (guard.stage_profile.clone(), guard.effects)
    }

    /// 次のステップを取得する。
    pub fn next_step(&self) -> IterStep<T> {
        let mut guard = self
            .core
            .state
            .lock()
            .expect("IterState poisoned during next_step()");
        guard.next_step()
    }

    /// 次の値のみを取り出す。
    pub fn next(&self) -> Option<T> {
        match self.next_step() {
            IterStep::Ready(value) => Some(value),
            _ => None,
        }
    }

    /// `ListCollector` を利用して永続リストへ収集する。
    pub fn collect_list(self) -> Result<CollectOutcome<List<T>>, CollectError> {
        self.collect_into_collector(ListCollector::new())
    }

    /// `VecCollector` を利用して可変ベクタへ収集する。
    pub fn collect_vec(self) -> Result<CollectOutcome<CoreVec<T>>, CollectError> {
        self.collect_into_collector(VecCollector::new())
    }

    /// `Iterator::fold` 相当の終端操作。
    pub fn fold<U, F>(self, init: U, mut f: F) -> U
    where
        F: FnMut(U, T) -> U,
    {
        let iter = self;
        let mut acc = init;
        loop {
            match iter.next_step() {
                IterStep::Ready(value) => {
                    acc = f(acc, value);
                }
                IterStep::Pending => continue,
                IterStep::Finished => return acc,
                IterStep::Error(_err) => return acc,
            }
        }
    }

    /// `Iterator::reduce` 相当の終端操作。
    pub fn reduce<F>(self, mut f: F) -> Option<T>
    where
        F: FnMut(T, T) -> T,
    {
        let iter = self;
        let mut accumulator: Option<T> = None;
        loop {
            match iter.next_step() {
                IterStep::Ready(value) => {
                    accumulator = Some(match accumulator.take() {
                        Some(current) => f(current, value),
                        None => value,
                    });
                }
                IterStep::Pending => continue,
                IterStep::Finished => return accumulator,
                IterStep::Error(_err) => return accumulator,
            }
        }
    }

    /// すべての要素が述語を満たすかを判定する。
    pub fn all<F>(self, mut predicate: F) -> bool
    where
        F: FnMut(&T) -> bool,
    {
        let iter = self;
        loop {
            match iter.next_step() {
                IterStep::Ready(value) => {
                    if !predicate(&value) {
                        return false;
                    }
                }
                IterStep::Pending => continue,
                IterStep::Finished => return true,
                IterStep::Error(_err) => return false,
            }
        }
    }

    /// 少なくとも 1 つの要素が述語を満たすかを判定する。
    pub fn any<F>(self, mut predicate: F) -> bool
    where
        F: FnMut(&T) -> bool,
    {
        let iter = self;
        loop {
            match iter.next_step() {
                IterStep::Ready(value) => {
                    if predicate(&value) {
                        return true;
                    }
                }
                IterStep::Pending => continue,
                IterStep::Finished => return false,
                IterStep::Error(_err) => return false,
            }
        }
    }

    /// 述語を満たす最初の要素を返す。
    pub fn find<F>(self, mut predicate: F) -> Option<T>
    where
        F: FnMut(&T) -> bool,
    {
        let iter = self;
        loop {
            match iter.next_step() {
                IterStep::Ready(value) => {
                    if predicate(&value) {
                        return Some(value);
                    }
                }
                IterStep::Pending => continue,
                IterStep::Finished => return None,
                IterStep::Error(_err) => return None,
            }
        }
    }

    /// `Result` で短絡する `fold`。
    pub fn try_fold<U, E, F>(self, init: U, mut f: F) -> Result<U, E>
    where
        F: FnMut(U, T) -> Result<U, E>,
    {
        let iter = self;
        let mut acc = init;
        loop {
            match iter.next_step() {
                IterStep::Ready(value) => {
                    acc = f(acc, value)?;
                }
                IterStep::Pending => continue,
                IterStep::Finished => return Ok(acc),
                IterStep::Error(_err) => return Ok(acc),
            }
        }
    }

    fn collect_into_collector<C, Output>(self, collector: C) -> Result<Output, C::Error>
    where
        C: Collector<T, Output>,
    {
        self.drain_into_collector(collector)
    }

    fn drain_into_collector<C, Output>(self, mut collector: C) -> Result<Output, C::Error>
    where
        C: Collector<T, Output>,
    {
        let iter = self;
        loop {
            match iter.next_step() {
                IterStep::Ready(value) => collector.push(value)?,
                IterStep::Pending => continue,
                IterStep::Finished => return Ok(collector.finish()),
                IterStep::Error(err) => return Err(collector.iter_error(err)),
            }
        }
    }
}

impl<T> IntoIterator for Iter<T> {
    type Item = T;
    type IntoIter = IterIntoIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        IterIntoIterator { iter: self }
    }
}

impl<T> Iterator for IterIntoIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<T> FromIterator<T> for Iter<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iterable: I) -> Self {
        Self::from_list(iterable.into_iter().collect::<Vec<T>>())
    }
}

#[cfg(feature = "core_numeric")]
impl Iter<f64> {
    /// 数値列を `NumericCollector` で収集するヘルパ。
    pub fn collect_numeric(self) -> Result<CollectOutcome<CoreVec<f64>>, CollectError> {
        ensure_numeric_metrics_stage(METRIC_STAGE_REQUIREMENT, "Iter::collect_numeric")?;
        self.collect_into_collector(NumericCollector::new())
    }
}

#[cfg(feature = "core_numeric")]
fn ensure_numeric_metrics_stage(
    requirement: crate::StageRequirement,
    source: &'static str,
) -> Result<(), CollectError> {
    let required_effects = metric_required_effects();
    MetricsStageGuard::verify(requirement, &required_effects)
        .map(|_| ())
        .map_err(|err| {
            let profile = CollectorStageProfile::for_kind(CollectorKind::Numeric);
            let snapshot = profile.snapshot(source);
            let audit = CollectorAuditTrail::new(
                CollectorKind::Numeric,
                snapshot,
                EffectLabels {
                    mem: false,
                    mutating: false,
                    debug: false,
                    async_pending: false,
                    audit: false,
                    cell: false,
                    rc: false,
                    unicode: false,
                    io: false,
                    io_blocking: false,
                    io_async: false,
                    security: false,
                    transfer: false,
                    fs_sync: false,
                    mem_bytes: 0,
                    predicate_calls: 0,
                    rc_ops: 0,
                    time: false,
                    time_calls: 0,
                    io_blocking_calls: 0,
                    io_async_calls: 0,
                    fs_sync_calls: 0,
                    security_events: 0,
                },
                CollectorEffectMarkers::default(),
            );
            CollectError::capability_denied(METRIC_CAPABILITY_ID, audit, err)
        })
}

impl<K, V> Iter<(K, V)> {
    /// `MapCollector` を利用してキー付きデータを永続マップへ収集する。
    pub fn collect_map(self) -> Result<CollectOutcome<Map<K, V>>, CollectError>
    where
        K: Ord + Clone + fmt::Debug + Serialize,
        V: Clone + Serialize,
    {
        self.collect_into_collector(MapCollector::new())
    }

    /// `TableCollector` を利用して挿入順序付きテーブルへ収集する。
    pub fn collect_table(self) -> Result<CollectOutcome<Table<K, V>>, CollectError>
    where
        K: std::cmp::Eq + std::hash::Hash + Clone + std::fmt::Debug,
    {
        self.collect_into_collector(TableCollector::new())
    }
}

impl<T> Iter<T>
where
    T: Ord + Clone + fmt::Debug + Serialize,
{
    /// `SetCollector` を利用して重複排除された集合へ収集する。
    pub fn collect_set(self) -> Result<CollectOutcome<Set<T>>, CollectError> {
        self.collect_into_collector(SetCollector::new())
    }
}

impl<T> Iter<T> {
    /// `CollectorAuditTrail` 由来の効果ラベルを `EffectSet` へ反映する。
    fn merge_collector_audit(&self, audit: &CollectorAuditTrail) {
        let mut guard = self
            .core
            .state
            .lock()
            .expect("IterState poisoned while merging collector audit");
        guard.effects.merge_labels(audit.effects);
    }
}

impl<T, E> Iter<Result<T, E>> {
    /// `Result` を要素に含む `Iter` を Collector へ短絡収集する。
    pub fn try_collect<C, Value>(self, collector: C) -> Result<Value, TryCollectError<E, C::Error>>
    where
        C: Collector<T, CollectOutcome<Value>, Error = CollectError>,
    {
        let iter = self;
        let mut bridge = CollectorBridge::new(&iter, collector);
        loop {
            match iter.next_step() {
                IterStep::Ready(result) => match result {
                    Ok(value) => {
                        if let Err(err) = bridge.push(value) {
                            bridge.record_error(&err);
                            return Err(TryCollectError::Collector(err));
                        }
                    }
                    Err(err) => return Err(TryCollectError::Item(err)),
                },
                IterStep::Pending => continue,
                IterStep::Finished => {
                    let (value, audit) = bridge.finalize();
                    iter.merge_collector_audit(&audit);
                    return Ok(value);
                }
                IterStep::Error(err) => return Err(TryCollectError::Iter(err)),
            }
        }
    }
}

/// `Iter` が共有する内部状態。
#[derive(Debug)]
pub struct IterState<T> {
    source: IterSource<T>,
    stage_profile: IteratorStageProfile,
    effects: EffectSet,
    driver: IterDriver<T>,
}

impl<T> IterState<T> {
    /// 新しい状態を生成する。
    pub fn new(source: IterSource<T>, stage_profile: IteratorStageProfile) -> Self {
        Self {
            source,
            stage_profile,
            effects: EffectSet::PURE,
            driver: IterDriver::Empty,
        }
    }

    /// 効果タグを上書きする（アダプタ連結時に使用）。
    pub fn with_effects(mut self, effects: EffectSet) -> Self {
        self.effects = effects;
        self
    }

    /// ドライバを設定する。
    pub(crate) fn with_driver(mut self, driver: IterDriver<T>) -> Self {
        self.driver = driver;
        self
    }

    /// 現在の `IterSource` を返す。
    pub fn source(&self) -> &IterSource<T> {
        &self.source
    }

    /// Stage/Capability 情報を返す。
    pub fn stage_profile(&self) -> &IteratorStageProfile {
        &self.stage_profile
    }

    /// 効果タグを取得する。
    pub fn effects(&self) -> EffectSet {
        self.effects
    }

    fn next_step(&mut self) -> IterStep<T> {
        self.driver.next_step(&mut self.effects)
    }
}

/// イテレータの生成元。
#[derive(Debug)]
pub enum IterSource<T> {
    /// `Iter::from_fn` などの種シーケンス。
    Seed(IterSeed<T>),
    /// 既存の `IterState` にアダプタを連結したもの。
    Adapter {
        label: &'static str,
        stage: IteratorStageProfile,
        _marker: PhantomData<T>,
    },
    /// イテレータが空であることを示すプレースホルダ。
    Empty,
}

impl<T> IterSource<T> {
    /// 表示用ラベルを返す。
    pub fn label(&self) -> Cow<'_, str> {
        match self {
            Self::Seed(seed) => Cow::Borrowed(seed.label()),
            Self::Adapter { label, .. } => Cow::Borrowed(label),
            Self::Empty => Cow::Borrowed("Iter::empty"),
        }
    }
}

/// シードベースの `Iter` を表す型。
#[derive(Debug)]
pub struct IterSeed<T> {
    label: &'static str,
    stage: IteratorStageProfile,
    driver: IterDriver<T>,
    effects: EffectSet,
}

impl<T> IterSeed<T> {
    /// 新しいシードを生成する。
    pub(crate) fn new(
        label: &'static str,
        stage: IteratorStageProfile,
        driver: IterDriver<T>,
        effects: EffectSet,
    ) -> Self {
        Self {
            label,
            stage,
            driver,
            effects,
        }
    }

    /// シード名を返す。
    pub fn label(&self) -> &'static str {
        self.label
    }

    /// Stage プロファイルを返す。
    pub fn stage_profile(&self) -> &IteratorStageProfile {
        &self.stage
    }

    fn effects(&self) -> EffectSet {
        self.effects
    }

    fn take_driver(&mut self) -> IterDriver<T> {
        std::mem::replace(&mut self.driver, IterDriver::Empty)
    }
}

/// バッファリング戦略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferStrategy {
    DropOldest,
    Grow,
}

/// `Iter::buffered` で発生しうるエラー。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IterBufferError {
    pub capacity: usize,
    pub strategy: BufferStrategy,
}

impl IterBufferError {
    pub fn capacity_overflow(capacity: usize, strategy: BufferStrategy) -> Self {
        Self { capacity, strategy }
    }
}

/// イテレータ操作全般のエラー。
#[derive(Debug)]
pub enum IterError {
    Buffer(IterBufferError),
}

impl IterError {
    pub fn buffer_overflow(capacity: usize, strategy: BufferStrategy) -> Self {
        Self::Buffer(IterBufferError::capacity_overflow(capacity, strategy))
    }
}

/// `Iter::try_collect` で伝播するエラー。
#[derive(Debug)]
pub enum TryCollectError<ItemError, CollectorError> {
    Item(ItemError),
    Collector(CollectorError),
    Iter(IterError),
}

/// `Iter` のステップ種別。
#[derive(Debug)]
pub enum IterStep<T> {
    Ready(T),
    Pending,
    Finished,
    Error(IterError),
}

impl<T> IterStep<T> {
    /// `Ready` であるかを判定する。
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// `Finished` であるかを判定する。
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Finished)
    }

    /// `Error` であるかを判定する。
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

/// Stage 要件の表現。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageRequirement {
    Exact(&'static str),
    AtLeast(&'static str),
}

impl StageRequirement {
    pub fn descriptor(&self) -> StageRequirementDescriptor {
        match self {
            StageRequirement::Exact(stage) => StageRequirementDescriptor {
                mode: "exact",
                stage,
            },
            StageRequirement::AtLeast(stage) => StageRequirementDescriptor {
                mode: "at_least",
                stage,
            },
        }
    }
}

/// Stage 要件の診断向け表現。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageRequirementDescriptor {
    pub mode: &'static str,
    pub stage: &'static str,
}

/// イテレータの種類。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IteratorKind {
    ArrayLike,
    CoreIter,
    OptionLike,
    ResultLike,
    PersistentCollection,
    Custom(String),
}

impl IteratorKind {
    pub fn capability_id(&self) -> Option<&'static str> {
        match self {
            IteratorKind::ArrayLike => Some("core.iter.array"),
            IteratorKind::CoreIter => Some("core.iter.core"),
            IteratorKind::OptionLike => Some("core.iter.option"),
            IteratorKind::ResultLike => Some("core.iter.result"),
            IteratorKind::PersistentCollection => Some("core.iter.persistent_collection"),
            IteratorKind::Custom(_) => None,
        }
    }

    pub fn default_requirement(&self) -> StageRequirement {
        match self {
            IteratorKind::ArrayLike => StageRequirement::Exact("stable"),
            IteratorKind::PersistentCollection => StageRequirement::Exact("stable"),
            _ => StageRequirement::AtLeast("beta"),
        }
    }

    pub fn default_actual(&self) -> &'static str {
        match self {
            IteratorKind::ArrayLike => "stable",
            IteratorKind::CoreIter | IteratorKind::OptionLike | IteratorKind::ResultLike => "beta",
            IteratorKind::PersistentCollection => "stable",
            IteratorKind::Custom(_) => "unknown",
        }
    }

    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            IteratorKind::ArrayLike => Cow::Borrowed("array_like"),
            IteratorKind::CoreIter => Cow::Borrowed("core_iter"),
            IteratorKind::OptionLike => Cow::Borrowed("option_like"),
            IteratorKind::ResultLike => Cow::Borrowed("result_like"),
            IteratorKind::Custom(label) => Cow::Owned(format!("custom:{label}")),
            IteratorKind::PersistentCollection => Cow::Borrowed("persistent_collection"),
        }
    }
}

/// Stage/Capability 情報。
#[derive(Debug, Clone)]
pub struct IteratorStageProfile {
    requirement: StageRequirement,
    actual: &'static str,
    capability: Option<&'static str>,
    kind: IteratorKind,
}

impl IteratorStageProfile {
    /// 既定設定で Stage 情報を構築する。
    pub fn for_kind(kind: IteratorKind) -> Self {
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

    /// Stage 要件を上書きする。
    pub fn with_requirement(mut self, requirement: StageRequirement) -> Self {
        self.requirement = requirement;
        self
    }

    /// 実際の Stage を指定する。
    pub fn with_actual(mut self, actual: &'static str) -> Self {
        self.actual = actual;
        self
    }

    /// 診断/監査向けのキーを生成する。
    pub fn snapshot(&self, source: impl Into<String>) -> IteratorStageSnapshot {
        IteratorStageSnapshot {
            required: self.requirement.descriptor(),
            actual: self.actual,
            capability: self.capability,
            kind: self.kind.as_str().into_owned(),
            source: source.into(),
        }
    }
}

/// Stage 情報を `Diagnostic`/`AuditEnvelope` へ転写するためのデータ。
#[derive(Debug, Clone)]
pub struct IteratorStageSnapshot {
    pub required: StageRequirementDescriptor,
    pub actual: &'static str,
    pub capability: Option<&'static str>,
    pub kind: String,
    pub source: String,
}

/// 効果ビット集合。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectSet {
    bits: u16,
    mem_bytes: usize,
    predicate_calls: usize,
    rc_ops: usize,
    time_calls: usize,
    io_blocking_ops: usize,
    io_async_ops: usize,
    fs_sync_ops: usize,
    security_events: usize,
}

impl EffectSet {
    const MUT_BIT: u16 = 0b0000_0001;
    const MEM_BIT: u16 = 0b0000_0010;
    const DEBUG_BIT: u16 = 0b0000_0100;
    const PENDING_BIT: u16 = 0b0000_1000;
    const AUDIT_BIT: u16 = 0b0001_0000;
    const CELL_BIT: u16 = 0b0010_0000;
    const RC_BIT: u16 = 0b0100_0000;
    const IO_BIT: u16 = 0b1000_0000;
    const TRANSFER_BIT: u16 = 0b1_0000_0000;
    const UNICODE_BIT: u16 = 0b10_0000_0000;
    const TIME_BIT: u16 = 0b100_0000_0000;
    const IO_BLOCKING_BIT: u16 = 0b1000_0000_0000;
    const IO_ASYNC_BIT: u16 = 0b1_0000_0000_0000;
    const SECURITY_BIT: u16 = 0b10_0000_0000_0000;
    const FS_SYNC_BIT: u16 = 0b100_0000_0000_0000;

    pub const PURE: Self = Self {
        bits: 0,
        mem_bytes: 0,
        predicate_calls: 0,
        rc_ops: 0,
        time_calls: 0,
        io_blocking_ops: 0,
        io_async_ops: 0,
        fs_sync_ops: 0,
        security_events: 0,
    };

    pub fn mark_mut(&mut self) {
        self.bits |= Self::MUT_BIT;
    }

    pub fn mark_cell(&mut self) {
        self.bits |= Self::CELL_BIT;
    }

    pub fn mark_rc(&mut self) {
        self.bits |= Self::RC_BIT;
        self.rc_ops = self.rc_ops.saturating_add(1);
    }

    pub fn release_rc(&mut self) {
        self.bits |= Self::RC_BIT;
    }

    pub fn mark_mem(&mut self) {
        self.bits |= Self::MEM_BIT;
    }

    pub fn mark_debug(&mut self) {
        self.bits |= Self::DEBUG_BIT;
    }

    pub fn mark_pending(&mut self) {
        self.bits |= Self::PENDING_BIT;
    }

    pub fn mark_audit(&mut self) {
        self.bits |= Self::AUDIT_BIT;
    }

    pub fn mark_io(&mut self) {
        self.bits |= Self::IO_BIT;
    }

    pub fn mark_io_blocking(&mut self) {
        self.record_io_blocking_calls(1);
    }

    pub fn record_io_blocking_calls(&mut self, calls: usize) {
        if calls == 0 {
            return;
        }
        self.io_blocking_ops = self.io_blocking_ops.saturating_add(calls);
        self.bits |= Self::IO_BLOCKING_BIT;
        self.mark_io();
    }

    pub fn mark_io_async(&mut self) {
        self.record_io_async_events(1);
    }

    pub fn record_io_async_events(&mut self, events: usize) {
        if events == 0 {
            return;
        }
        self.io_async_ops = self.io_async_ops.saturating_add(events);
        self.bits |= Self::IO_ASYNC_BIT;
        self.mark_io();
    }

    pub fn mark_transfer(&mut self) {
        self.bits |= Self::TRANSFER_BIT;
    }

    pub fn mark_unicode(&mut self) {
        self.bits |= Self::UNICODE_BIT;
    }

    pub fn mark_fs_sync(&mut self) {
        self.record_fs_sync_calls(1);
    }

    pub fn record_fs_sync_calls(&mut self, calls: usize) {
        if calls == 0 {
            return;
        }
        self.fs_sync_ops = self.fs_sync_ops.saturating_add(calls);
        self.bits |= Self::FS_SYNC_BIT;
        self.mark_io_blocking();
    }

    pub fn mark_time(&mut self) {
        self.bits |= Self::TIME_BIT;
    }

    pub fn mark_security(&mut self) {
        self.record_security_events(1);
    }

    pub fn record_security_events(&mut self, events: usize) {
        if events == 0 {
            return;
        }
        self.security_events = self.security_events.saturating_add(events);
        self.bits |= Self::SECURITY_BIT;
    }

    pub fn record_predicate_call(&mut self) {
        self.predicate_calls = self.predicate_calls.saturating_add(1);
    }

    pub fn record_mem_bytes(&mut self, bytes: usize) {
        self.mem_bytes = self.mem_bytes.saturating_add(bytes);
    }

    /// 複数回の述語呼び出しをまとめて追加する。
    pub fn record_predicate_calls(&mut self, calls: usize) {
        self.predicate_calls = self.predicate_calls.saturating_add(calls);
    }

    /// 複数の参照カウント操作を記録する。
    pub fn record_rc_ops(&mut self, ops: usize) {
        self.rc_ops = self.rc_ops.saturating_add(ops);
    }

    pub fn record_time_calls(&mut self, calls: usize) {
        if calls == 0 {
            return;
        }
        self.time_calls = self.time_calls.saturating_add(calls);
        self.mark_time();
    }

    pub fn record_time_call(&mut self) {
        self.record_time_calls(1);
    }

    pub fn with_mut(self) -> Self {
        Self {
            bits: self.bits | Self::MUT_BIT,
            mem_bytes: self.mem_bytes,
            predicate_calls: self.predicate_calls,
            rc_ops: self.rc_ops,
            time_calls: self.time_calls,
            io_blocking_ops: self.io_blocking_ops,
            io_async_ops: self.io_async_ops,
            fs_sync_ops: self.fs_sync_ops,
            security_events: self.security_events,
        }
    }

    pub fn with_mem(self) -> Self {
        Self {
            bits: self.bits | Self::MEM_BIT,
            mem_bytes: self.mem_bytes,
            predicate_calls: self.predicate_calls,
            rc_ops: self.rc_ops,
            time_calls: self.time_calls,
            io_blocking_ops: self.io_blocking_ops,
            io_async_ops: self.io_async_ops,
            fs_sync_ops: self.fs_sync_ops,
            security_events: self.security_events,
        }
    }

    pub fn with_debug(self) -> Self {
        Self {
            bits: self.bits | Self::DEBUG_BIT,
            mem_bytes: self.mem_bytes,
            predicate_calls: self.predicate_calls,
            rc_ops: self.rc_ops,
            time_calls: self.time_calls,
            io_blocking_ops: self.io_blocking_ops,
            io_async_ops: self.io_async_ops,
            fs_sync_ops: self.fs_sync_ops,
            security_events: self.security_events,
        }
    }

    pub fn with_pending(self) -> Self {
        Self {
            bits: self.bits | Self::PENDING_BIT,
            mem_bytes: self.mem_bytes,
            predicate_calls: self.predicate_calls,
            rc_ops: self.rc_ops,
            time_calls: self.time_calls,
            io_blocking_ops: self.io_blocking_ops,
            io_async_ops: self.io_async_ops,
            fs_sync_ops: self.fs_sync_ops,
            security_events: self.security_events,
        }
    }

    pub fn with_predicate_calls(mut self, calls: usize) -> Self {
        self.predicate_calls = self.predicate_calls.saturating_add(calls);
        self
    }

    pub fn predicate_calls(self) -> usize {
        self.predicate_calls
    }

    pub fn with_mem_bytes(mut self, bytes: usize) -> Self {
        self.mem_bytes = self.mem_bytes.saturating_add(bytes);
        self
    }

    pub fn mem_bytes(self) -> usize {
        self.mem_bytes
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
            mem_bytes: self.mem_bytes.saturating_add(other.mem_bytes),
            predicate_calls: self.predicate_calls.saturating_add(other.predicate_calls),
            rc_ops: self.rc_ops.saturating_add(other.rc_ops),
            time_calls: self.time_calls.saturating_add(other.time_calls),
            io_blocking_ops: self.io_blocking_ops.saturating_add(other.io_blocking_ops),
            io_async_ops: self.io_async_ops.saturating_add(other.io_async_ops),
            fs_sync_ops: self.fs_sync_ops.saturating_add(other.fs_sync_ops),
            security_events: self.security_events.saturating_add(other.security_events),
        }
    }

    /// `EffectLabels` を `EffectSet` に反映する。
    pub fn merge_labels(&mut self, labels: EffectLabels) {
        if labels.mem {
            self.mark_mem();
        }
        if labels.mutating {
            self.mark_mut();
        }
        if labels.debug {
            self.mark_debug();
        }
        if labels.async_pending {
            self.mark_pending();
        }
        if labels.audit {
            self.mark_audit();
        }
        if labels.cell {
            self.mark_cell();
        }
        if labels.rc || labels.rc_ops > 0 {
            self.bits |= Self::RC_BIT;
        }
        if labels.io {
            self.mark_io();
        }
        if labels.io_blocking_calls > 0 {
            self.record_io_blocking_calls(labels.io_blocking_calls);
        } else if labels.io_blocking {
            self.mark_io_blocking();
        }
        if labels.io_async_calls > 0 {
            self.record_io_async_events(labels.io_async_calls);
        } else if labels.io_async {
            self.mark_io_async();
        }
        if labels.transfer {
            self.mark_transfer();
        }
        if labels.fs_sync_calls > 0 {
            self.record_fs_sync_calls(labels.fs_sync_calls);
        } else if labels.fs_sync {
            self.mark_fs_sync();
        }
        if labels.unicode {
            self.mark_unicode();
        }
        if labels.time {
            self.mark_time();
        }
        if labels.security_events > 0 {
            self.record_security_events(labels.security_events);
        } else if labels.security {
            self.mark_security();
        }

        self.record_mem_bytes(labels.mem_bytes);
        self.record_predicate_calls(labels.predicate_calls);
        self.record_rc_ops(labels.rc_ops);
        self.record_time_calls(labels.time_calls);
    }

    pub fn contains_mut(self) -> bool {
        self.bits & Self::MUT_BIT != 0
    }

    pub fn contains_mem(self) -> bool {
        self.bits & Self::MEM_BIT != 0
    }

    pub fn contains_debug(self) -> bool {
        self.bits & Self::DEBUG_BIT != 0
    }

    pub fn contains_pending(self) -> bool {
        self.bits & Self::PENDING_BIT != 0
    }

    pub fn contains_audit(self) -> bool {
        self.bits & Self::AUDIT_BIT != 0
    }

    pub fn contains_cell(self) -> bool {
        self.bits & Self::CELL_BIT != 0
    }

    pub fn contains_rc(self) -> bool {
        self.bits & Self::RC_BIT != 0
    }

    pub fn contains_io(self) -> bool {
        self.bits & Self::IO_BIT != 0
    }

    pub fn contains_io_blocking(self) -> bool {
        self.bits & Self::IO_BLOCKING_BIT != 0
    }

    pub fn contains_io_async(self) -> bool {
        self.bits & Self::IO_ASYNC_BIT != 0
    }

    pub fn contains_transfer(self) -> bool {
        self.bits & Self::TRANSFER_BIT != 0
    }

    pub fn contains_unicode(self) -> bool {
        self.bits & Self::UNICODE_BIT != 0
    }

    pub fn contains_time(self) -> bool {
        self.bits & Self::TIME_BIT != 0
    }

    pub fn contains_security(self) -> bool {
        self.bits & Self::SECURITY_BIT != 0
    }

    pub fn contains_fs_sync(self) -> bool {
        self.bits & Self::FS_SYNC_BIT != 0
    }

    pub fn to_labels(self) -> EffectLabels {
        EffectLabels {
            mem: self.contains_mem(),
            mutating: self.contains_mut(),
            debug: self.contains_debug(),
            async_pending: self.contains_pending(),
            audit: self.contains_audit(),
            cell: self.contains_cell(),
            rc: self.contains_rc(),
            unicode: self.contains_unicode(),
            io: self.contains_io(),
            io_blocking: self.contains_io_blocking(),
            io_async: self.contains_io_async(),
            security: self.contains_security(),
            fs_sync: self.contains_fs_sync(),
            transfer: self.contains_transfer(),
            time: self.contains_time(),
            mem_bytes: self.mem_bytes,
            predicate_calls: self.predicate_calls,
            rc_ops: self.rc_ops,
            time_calls: self.time_calls,
            io_blocking_calls: self.io_blocking_ops,
            io_async_calls: self.io_async_ops,
            fs_sync_calls: self.fs_sync_ops,
            security_events: self.security_events,
        }
    }
}

/// `collect-iterator-audit-metrics.py` へ渡す効果ラベル。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectLabels {
    pub mem: bool,
    pub mutating: bool,
    pub debug: bool,
    pub async_pending: bool,
    pub audit: bool,
    pub cell: bool,
    pub rc: bool,
    pub unicode: bool,
    pub io: bool,
    pub io_blocking: bool,
    pub io_async: bool,
    pub security: bool,
    pub transfer: bool,
    pub fs_sync: bool,
    pub mem_bytes: usize,
    pub predicate_calls: usize,
    pub rc_ops: usize,
    pub time: bool,
    pub time_calls: usize,
    pub io_blocking_calls: usize,
    pub io_async_calls: usize,
    pub fs_sync_calls: usize,
    pub security_events: usize,
}

pub(crate) enum IterDriver<T> {
    Static(VecDeque<T>),
    Stepper(Box<dyn FnMut(&mut EffectSet) -> IterStep<T> + Send + 'static>),
    Empty,
}

impl<T> fmt::Debug for IterDriver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(_) => f.write_str("IterDriver::Static(..)"),
            Self::Stepper(_) => f.write_str("IterDriver::Stepper(..)"),
            Self::Empty => f.write_str("IterDriver::Empty"),
        }
    }
}

impl<T> IterDriver<T> {
    fn from_vec(vec: Vec<T>) -> Self {
        Self::Static(VecDeque::from(vec))
    }

    fn stepper<F>(f: F) -> Self
    where
        F: FnMut(&mut EffectSet) -> IterStep<T> + Send + 'static,
    {
        Self::Stepper(Box::new(f))
    }

    fn next_step(&mut self, effects: &mut EffectSet) -> IterStep<T> {
        match self {
            IterDriver::Static(buffer) => buffer
                .pop_front()
                .map(IterStep::Ready)
                .unwrap_or(IterStep::Finished),
            IterDriver::Stepper(stepper) => (stepper)(effects),
            IterDriver::Empty => IterStep::Finished,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::collectors::CollectErrorKind;
    use super::*;

    fn iterator_error_iter() -> Iter<i64> {
        let stage = IteratorStageProfile::for_kind(IteratorKind::CoreIter);
        let driver = IterDriver::stepper(|_effects| {
            IterStep::Error(IterError::buffer_overflow(4, BufferStrategy::DropOldest))
        });
        let seed = IterSeed::new(
            "iter.tests.iterator_error",
            stage.clone(),
            driver,
            EffectSet::PURE,
        );
        Iter::from_seed(seed)
    }

    #[test]
    fn collect_vec_propagates_iterator_errors() {
        let iter = iterator_error_iter();
        let err = iter
            .collect_vec()
            .expect_err("iterator error should propagate to collector");
        assert!(
            matches!(err.kind(), CollectErrorKind::IteratorFailure),
            "expected iterator failure, got {:?}",
            err.kind()
        );
    }

    #[cfg(feature = "core_numeric")]
    #[test]
    fn ensure_numeric_metrics_stage_reports_capability_error() {
        let error = super::ensure_numeric_metrics_stage(
            crate::StageRequirement::Exact(crate::StageId::Beta),
            "iter.tests.collect_numeric",
        )
        .expect_err("beta requirement should fail for metrics");
        assert!(
            matches!(error.kind(), CollectErrorKind::CapabilityDenied),
            "expected capability denied error, got {:?}",
            error.kind()
        );
    }
}
