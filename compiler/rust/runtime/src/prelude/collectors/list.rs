//! `ListCollector` と `List` の最小実装。
//! `effect = @pure` の再現と Stage/Marker の出力を担保する雛形。

use super::super::iter::EffectLabels;
use super::{
    CollectError, CollectOutcome, Collector, CollectorAuditTrail, CollectorEffectMarkers,
    CollectorKind, CollectorStageProfile,
};

const PURE_EFFECTS: EffectLabels = EffectLabels {
    mem: false,
    mutating: false,
    debug: false,
    async_pending: false,
    mem_bytes: 0,
};

/// 永続 `List` 型の雛形。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List<T> {
    elements: Vec<T>,
}

impl<T> List<T> {
    fn from_vec(elements: Vec<T>) -> Self {
        Self { elements }
    }

    pub fn into_vec(self) -> Vec<T> {
        self.elements
    }

    pub fn as_slice(&self) -> &[T] {
        &self.elements
    }
}

/// `ListCollector` は `@pure` に従い、Stage 実装を `stable` に固定する。
pub struct ListCollector<T> {
    buffer: Vec<T>,
    stage_profile: CollectorStageProfile,
    effects: EffectLabels,
    markers: CollectorEffectMarkers,
}

impl<T> ListCollector<T> {
    fn audit_trail(&self, source: &'static str) -> CollectorAuditTrail {
        CollectorAuditTrail::new(
            CollectorKind::List,
            self.stage_profile.snapshot(source),
            self.effects,
            self.markers,
        )
    }
}

impl<T> Collector<T, CollectOutcome<List<T>>> for ListCollector<T> {
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            buffer: Vec::new(),
            stage_profile: CollectorStageProfile::for_kind(CollectorKind::List),
            effects: PURE_EFFECTS,
            markers: CollectorEffectMarkers::default(),
        }
    }

    fn with_capacity(capacity: usize) -> Self
    where
        Self: Sized,
    {
        let mut collector = Self::new();
        collector.buffer.reserve(capacity);
        collector
    }

    fn push(&mut self, value: T) -> Result<(), Self::Error> {
        self.buffer.push(value);
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<List<T>>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        let audit = self.audit_trail("ListCollector::finish");
        let list = List::from_vec(self.buffer);
        CollectOutcome::new(list, audit)
    }
}
