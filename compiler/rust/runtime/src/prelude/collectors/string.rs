//! `StringCollector` の雛形。UTF-8 バッファを構築しつつ `effect {mem}` を記録する。

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
};

pub struct StringCollector {
    buffer: String,
    stage_profile: CollectorStageProfile,
    effects: EffectLabels,
    markers: CollectorEffectMarkers,
}

impl StringCollector {
    fn audit_trail(&self, source: &'static str) -> CollectorAuditTrail {
        CollectorAuditTrail::new(
            CollectorKind::String,
            self.stage_profile.snapshot(source),
            self.effects,
            self.markers,
        )
    }
}

impl Collector<char, CollectOutcome<String>> for StringCollector {
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            buffer: String::new(),
            stage_profile: CollectorStageProfile::for_kind(CollectorKind::String),
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

    fn push(&mut self, value: char) -> Result<(), Self::Error> {
        self.buffer.push(value);
        self.effects.mem = true;
        self.effects.mutating = true;
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

    fn finish(mut self) -> CollectOutcome<String>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        self.effects.mem = true;
        let audit = self.audit_trail("StringCollector::finish");
        CollectOutcome::new(self.buffer, audit)
    }
}
