//! `VecCollector` の雛形実装。`effect {mut, mem}` と Stage 実装を担保する。

use super::super::iter::{EffectLabels, IterError};
use super::{
    CollectError, CollectErrorKind, CollectOutcome, Collector, CollectorAuditTrail,
    CollectorEffectMarkers, CollectorKind, CollectorStageProfile,
};
use crate::collections::mutable::vec::error::map_try_reserve_error;
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

/// 可変バッファを返す `VecCollector`。
pub struct VecCollector<T> {
    buffer: CoreVec<T>,
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

    fn record_mem_bytes(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.effects.mem = true;
        self.effects.mem_bytes = self
            .effects
            .mem_bytes
            .saturating_add(CoreVec::<T>::bytes_for(count));
    }

    fn ensure_buffer_mem_bytes(&mut self) {
        let len = self.buffer.len();
        if len == 0 {
            return;
        }
        let required = CoreVec::<T>::bytes_for(len);
        if self.effects.mem_bytes < required {
            self.effects.mem_bytes = required;
        }
    }
}

impl<T> Collector<T, CollectOutcome<CoreVec<T>>> for VecCollector<T> {
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            buffer: CoreVec::new(),
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
        collector.buffer = CoreVec::with_capacity(capacity);
        collector.effects.mem = true;
        collector.record_mem_bytes(capacity);
        if capacity > 0 {
            collector.markers.record_mem_reservation(capacity);
        }
        collector
    }

    fn push(&mut self, value: T) -> Result<(), Self::Error> {
        self.buffer.push(value);
        self.effects.mutating = true;
        self.record_mem_bytes(1);
        Ok(())
    }

    fn reserve(&mut self, additional: usize) -> Result<(), Self::Error> {
        if additional == 0 {
            return Ok(());
        }
        self.buffer.try_reserve(additional).map_err(|err| {
            let audit = self.audit_trail("VecCollector::reserve");
            map_try_reserve_error(audit, "VecCollector::reserve", err)
        })?;
        self.effects.mutating = true;
        self.record_mem_bytes(additional);
        self.markers.record_reserve(additional);
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<CoreVec<T>>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        self.effects.mem = true;
        self.ensure_buffer_mem_bytes();
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
