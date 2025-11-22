//! `VecCollector` の雛形実装。`effect {mut, mem}` と Stage 実装を担保する。

use super::super::iter::{EffectLabels, IterError};
use super::{
    CollectError, CollectErrorKind, CollectOutcome, Collector, CollectorAuditTrail,
    CollectorEffectMarkers, CollectorKind, CollectorStageProfile,
};

const PURE_EFFECTS: EffectLabels = EffectLabels {
    mem: false,
    mutating: false,
    debug: false,
    async_pending: false,
    audit: false,
    mem_bytes: 0,
    predicate_calls: 0,
};

/// 可変バッファを返す `VecCollector`。
pub struct VecCollector<T> {
    buffer: Vec<T>,
    stage_profile: CollectorStageProfile,
    effects: EffectLabels,
    markers: CollectorEffectMarkers,
}

impl<T> VecCollector<T> {
    fn audit_trail(&self, source: &'static str) -> CollectorAuditTrail {
        CollectorAuditTrail::new(
            CollectorKind::Vec,
            self.stage_profile.snapshot(source),
            self.effects,
            self.markers,
        )
    }
}

impl<T> Collector<T, CollectOutcome<Vec<T>>> for VecCollector<T> {
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            buffer: Vec::new(),
            stage_profile: CollectorStageProfile::for_kind(CollectorKind::Vec),
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
        collector.effects.mem = true;
        collector.markers.record_mem_reservation(capacity);
        collector
    }

    fn push(&mut self, value: T) -> Result<(), Self::Error> {
        self.buffer.push(value);
        self.effects.mutating = true;
        self.effects.mem = true;
        Ok(())
    }

    fn reserve(&mut self, additional: usize) -> Result<(), Self::Error> {
        if additional > 0 {
            self.markers.record_reserve(additional);
        }
        self.buffer.reserve(additional);
        self.effects.mem = true;
        self.effects.mutating = true;
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<Vec<T>>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        self.effects.mem = true;
        let audit = self.audit_trail("VecCollector::finish");
        CollectOutcome::new(self.buffer, audit)
    }

    fn iter_error(self, error: IterError) -> Self::Error
    where
        Self: Sized,
    {
        let audit = self.audit_trail("VecCollector::iter_error");
        CollectError::new(
            CollectErrorKind::IteratorFailure,
            "iterator source reported an error during VecCollector::collect",
            audit,
        )
        .with_detail(format!("{error:?}"))
    }
}
