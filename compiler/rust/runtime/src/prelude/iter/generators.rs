#![allow(dead_code)]

//! `Iter` の生成 API を分離して `Core.Iter` の公開インターフェイスを整理するモジュール。
//! - WBS 3.1c-F1 に従い `IterState`/`IterSeed`/`IterSource` を crate 内部で共有しつつ、`Iter` 公開 API を
//!   `generators.rs` へ集約する。

use std::sync::{Arc, Mutex};

use super::{
    EffectLabels, EffectSet, Iter, IterCore, IterSeed, IterSource, IterState, IteratorKind,
    IteratorStageProfile, IteratorStageSnapshot,
};

#[derive(Debug, Clone)]
pub struct IterStepMetadata {
    pub stage_snapshot: IteratorStageSnapshot,
    pub effect_set: EffectSet,
    pub effect_labels: Option<EffectLabels>,
}

impl IterStepMetadata {
    pub fn new(stage_snapshot: IteratorStageSnapshot) -> Self {
        Self {
            stage_snapshot,
            effect_set: EffectSet::PURE,
            effect_labels: None,
        }
    }

    pub fn with_effects(mut self, effects: EffectSet) -> Self {
        self.effect_set = effects;
        self
    }
}

/// `IterStepMetadata` に `EffectLabels` を注入し、`collect-iterator-audit-metrics.py` で観測可能にする。
pub fn attach_effects(step: &mut IterStepMetadata) {
    step.effect_labels = Some(step.effect_set.to_labels());
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

    /// `IterSeed` を `Iter` に変換するヘルパ。
    pub(crate) fn from_seed(seed: IterSeed<T>) -> Self {
        let stage_profile = seed.stage_profile().clone();
        Self::from_state(IterState::new(IterSource::Seed(seed), stage_profile))
    }

    /// 任意の `IterSource` を `Iter` に変換するための最小ヘルパ。
    pub(crate) fn with_source(source: IterSource<T>, stage_profile: IteratorStageProfile) -> Self {
        Self::from_state(IterState::new(source, stage_profile))
    }

    /// 空の `Iter` を生成する（`IterSource::Empty`）。
    pub fn empty() -> Self {
        let stage_profile = IteratorStageProfile::for_kind(IteratorKind::CoreIter);
        Self::with_source(IterSource::Empty, stage_profile)
    }
}
