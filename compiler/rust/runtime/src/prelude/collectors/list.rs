//! `ListCollector` と 永続 `List` 実装の結合ポイント。
//! `effect = @pure` の再現と Stage/Marker の出力を担保しつつ、
//! `runtime/src/collections` 配下の finger tree ベース実装を差し込む。

use std::mem;

use super::super::iter::{EffectLabels, IterError};
use super::{
    CollectError, CollectErrorKind, CollectOutcome, Collector, CollectorAuditTrail,
    CollectorEffectMarkers, CollectorKind, CollectorStageProfile,
};
pub use crate::collections::persistent::list::List;

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

/// `ListCollector` は `@pure` に従い、Stage 実装を `stable` に固定する。
pub struct ListCollector<T> {
    list: List<T>,
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
            list: List::empty(),
            stage_profile: CollectorStageProfile::for_kind(CollectorKind::List),
            effects: PURE_EFFECTS,
            markers: CollectorEffectMarkers::default(),
        }
    }

    fn with_capacity(capacity: usize) -> Self
    where
        Self: Sized,
    {
        let _ = capacity;
        Self::new()
    }

    fn push(&mut self, value: T) -> Result<(), Self::Error> {
        self.list = self.list.push_back(value);
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<List<T>>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        let bytes = self.list.len().saturating_mul(mem::size_of::<T>());
        if bytes > 0 {
            self.effects.mem = true;
            self.effects.mem_bytes = self.effects.mem_bytes.saturating_add(bytes);
            self.markers.record_mem_reservation(bytes);
        }
        let audit = self.audit_trail("ListCollector::finish");
        CollectOutcome::new(self.list, audit)
    }

    fn iter_error(self, error: IterError) -> Self::Error
    where
        Self: Sized,
    {
        let audit = self.audit_trail("ListCollector::iter_error");
        CollectError::new(
            CollectErrorKind::IteratorFailure,
            "iterator source reported an error during ListCollector::collect",
            audit,
        )
        .with_detail(format!("{error:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::vec::VecCollector;
    use super::*;

    #[test]
    fn list_collector_roundtrip_and_vec_interop() {
        let mut list_collector = ListCollector::new();
        list_collector.push(1).unwrap();
        list_collector.push(2).unwrap();
        list_collector.push(3).unwrap();

        let (list, _) = list_collector.finish().into_parts();
        assert_eq!(list.to_vec(), vec![1, 2, 3]);

        let mut vec_collector = VecCollector::new();
        for value in list.iter() {
            vec_collector.push(value).unwrap();
        }
        let (core_vec, _) = vec_collector.finish().into_parts();
        assert_eq!(core_vec.into_inner(), vec![1, 2, 3]);
    }

    #[test]
    fn list_map_works_with_collector_output() {
        let mut collector = ListCollector::new();
        for value in 0..5 {
            collector.push(value).unwrap();
        }
        let (list, _) = collector.finish().into_parts();
        let squared = list.map(|value| value * value);
        assert_eq!(squared.to_vec(), vec![0, 1, 4, 9, 16]);
        let sum = squared.fold(0, |acc, value| acc + value);
        assert_eq!(sum, 30);
    }
}
