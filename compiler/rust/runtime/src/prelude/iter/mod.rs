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
    marker::PhantomData,
    sync::{Arc, Mutex},
};

mod generators;
pub use generators::*;

/// 遅延列 `Iter<T>` の共有ハンドル。
#[derive(Clone, Debug)]
pub struct Iter<T> {
    core: Arc<IterCore<T>>,
}

#[derive(Debug)]
struct IterCore<T> {
    state: Mutex<IterState<T>>,
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
    pub fn with_driver(mut self, driver: IterDriver<T>) -> Self {
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

    fn metadata(&self) -> IterStepMetadata {
        let snapshot = self
            .stage_profile
            .snapshot(self.source.label().into_owned());
        IterStepMetadata::new(snapshot).with_effects(self.effects)
    }

    fn next_step(&mut self) -> IterStep<T> {
        self.driver.next_step()
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
    pub fn new(
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

/// `Iter` のステップ種別。
#[derive(Debug)]
pub enum IterStep<T> {
    Ready(T),
    Pending,
    Finished,
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
    Custom(String),
}

impl IteratorKind {
    pub fn capability_id(&self) -> Option<&'static str> {
        match self {
            IteratorKind::ArrayLike => Some("core.iter.array"),
            IteratorKind::CoreIter => Some("core.iter.core"),
            IteratorKind::OptionLike => Some("core.iter.option"),
            IteratorKind::ResultLike => Some("core.iter.result"),
            IteratorKind::Custom(_) => None,
        }
    }

    pub fn default_requirement(&self) -> StageRequirement {
        match self {
            IteratorKind::ArrayLike => StageRequirement::Exact("stable"),
            _ => StageRequirement::AtLeast("beta"),
        }
    }

    pub fn default_actual(&self) -> &'static str {
        match self {
            IteratorKind::ArrayLike => "stable",
            IteratorKind::CoreIter | IteratorKind::OptionLike | IteratorKind::ResultLike => "beta",
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
    bits: u8,
}

impl EffectSet {
    const MUT_BIT: u8 = 0b0001;
    const MEM_BIT: u8 = 0b0010;
    const DEBUG_BIT: u8 = 0b0100;
    const PENDING_BIT: u8 = 0b1000;

    pub const PURE: Self = Self { bits: 0 };

    pub fn with_mut(self) -> Self {
        Self {
            bits: self.bits | Self::MUT_BIT,
        }
    }

    pub fn with_mem(self) -> Self {
        Self {
            bits: self.bits | Self::MEM_BIT,
        }
    }

    pub fn with_debug(self) -> Self {
        Self {
            bits: self.bits | Self::DEBUG_BIT,
        }
    }

    pub fn with_pending(self) -> Self {
        Self {
            bits: self.bits | Self::PENDING_BIT,
        }
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

    pub fn to_labels(self) -> EffectLabels {
        EffectLabels {
            mem: self.contains_mem(),
            mutating: self.contains_mut(),
            debug: self.contains_debug(),
            async_pending: self.contains_pending(),
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
}

enum IterDriver<T> {
    Static(VecDeque<T>),
    Stepper(Box<dyn FnMut() -> IterStep<T> + Send + 'static>),
    Empty,
}

impl<T> IterDriver<T> {
    fn from_vec(vec: Vec<T>) -> Self {
        Self::Static(VecDeque::from(vec))
    }

    fn stepper<F>(f: F) -> Self
    where
        F: FnMut() -> IterStep<T> + Send + 'static,
    {
        Self::Stepper(Box::new(f))
    }

    fn next_step(&mut self) -> IterStep<T> {
        match self {
            IterDriver::Static(buffer) => buffer
                .pop_front()
                .map(IterStep::Ready)
                .unwrap_or(IterStep::Finished),
            IterDriver::Stepper(stepper) => (stepper)(),
            IterDriver::Empty => IterStep::Finished,
        }
    }
}
