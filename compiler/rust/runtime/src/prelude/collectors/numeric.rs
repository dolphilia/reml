use super::super::iter::{EffectLabels, IterError};
use super::{
    CollectError, CollectErrorKind, CollectOutcome, Collector, CollectorAuditTrail,
    CollectorEffectMarkers, CollectorKind, CollectorStageProfile,
};
use crate::collections::mutable::CoreVec;

const PURE_EFFECTS: EffectLabels = EffectLabels {
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
};

/// 数値シーケンス用の Collector。効果タグと Stage 情報を `Core.Numeric`
/// の計測で利用できるよう固定する。
pub struct NumericCollector {
    values: CoreVec<f64>,
    stage_profile: CollectorStageProfile,
    effects: EffectLabels,
    markers: CollectorEffectMarkers,
}

impl NumericCollector {
    fn audit_trail(&self, source: &'static str) -> CollectorAuditTrail {
        CollectorAuditTrail::new(
            CollectorKind::Numeric,
            self.stage_profile.snapshot(source),
            self.effects,
            self.markers,
        )
    }

    fn record_mem_bytes(&mut self, elems: usize) {
        if elems == 0 {
            return;
        }
        self.effects.mem = true;
        self.effects.mem_bytes = self
            .effects
            .mem_bytes
            .saturating_add(CoreVec::<f64>::bytes_for(elems));
    }
}

impl Collector<f64, CollectOutcome<CoreVec<f64>>> for NumericCollector {
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            values: CoreVec::new(),
            stage_profile: CollectorStageProfile::for_kind(CollectorKind::Numeric),
            effects: PURE_EFFECTS,
            markers: CollectorEffectMarkers::default(),
        }
    }

    fn with_capacity(capacity: usize) -> Self
    where
        Self: Sized,
    {
        let mut collector = Self::new();
        if capacity > 0 {
            collector.markers.record_mem_reservation(capacity);
        }
        collector.values = CoreVec::with_capacity(capacity);
        collector.record_mem_bytes(capacity);
        collector
    }

    fn push(&mut self, value: f64) -> Result<(), Self::Error> {
        self.values.push(value);
        self.effects.mutating = true;
        self.record_mem_bytes(1);
        Ok(())
    }

    fn reserve(&mut self, additional: usize) -> Result<(), Self::Error> {
        if additional == 0 {
            return Ok(());
        }
        self.values.reserve(additional);
        self.markers.record_reserve(additional);
        self.record_mem_bytes(additional);
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<CoreVec<f64>>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        let audit = self.audit_trail("NumericCollector::finish");
        CollectOutcome::new(self.values, audit)
    }

    fn iter_error(self, error: IterError) -> Self::Error
    where
        Self: Sized,
    {
        let audit = self.audit_trail("NumericCollector::iter_error");
        CollectError::new(
            CollectErrorKind::IteratorFailure,
            "iterator source reported an error during NumericCollector::collect",
            audit,
        )
        .with_detail(format!("{error:?}"))
    }
}
