//! `MapCollector` の実装。永続 `Map` を `Collector` へ接続する。

use std::{fmt::Debug, mem};

use serde::Serialize;

use super::super::iter::{EffectLabels, IterError};
use super::{
    CollectError, CollectErrorKind, CollectOutcome, Collector, CollectorAuditTrail,
    CollectorEffectMarkers, CollectorKind, CollectorStageProfile,
};
use crate::collections::persistent::btree::PersistentMap;

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

/// Collector から公開する `Map` 型。
pub type Map<K, V> = PersistentMap<K, V>;

pub struct MapCollector<K, V> {
    storage: Map<K, V>,
    stage_profile: CollectorStageProfile,
    effects: EffectLabels,
    markers: CollectorEffectMarkers,
}

impl<K: Ord, V> MapCollector<K, V> {
    fn audit_trail(&self, source: &'static str) -> CollectorAuditTrail {
        CollectorAuditTrail::new(
            CollectorKind::Map,
            self.stage_profile.snapshot(source),
            self.effects,
            self.markers,
        )
    }

    fn duplicate_error(&self, key: &K) -> CollectError
    where
        K: Debug,
    {
        CollectError::new(
            CollectErrorKind::DuplicateKey,
            format!("duplicate key: {key:?}"),
            self.audit_trail("MapCollector::push"),
        )
        .with_error_key(format!("{key:?}"))
    }
}

impl<K, V> Collector<(K, V), CollectOutcome<Map<K, V>>> for MapCollector<K, V>
where
    K: Ord + Clone + Debug + Serialize,
    V: Clone + Serialize,
{
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            storage: Map::new(),
            stage_profile: CollectorStageProfile::for_kind(CollectorKind::Map),
            effects: PURE_EFFECTS,
            markers: CollectorEffectMarkers::default(),
        }
    }

    fn with_capacity(_capacity: usize) -> Self
    where
        Self: Sized,
    {
        Self::new()
    }

    fn push(&mut self, value: (K, V)) -> Result<(), Self::Error> {
        let (key, value) = value;
        if self.storage.contains_key(&key) {
            return Err(self.duplicate_error(&key));
        }
        let entry_bytes = mem::size_of::<K>().saturating_add(mem::size_of::<V>());
        self.effects.mem = true;
        if entry_bytes > 0 {
            self.effects.mem_bytes = self.effects.mem_bytes.saturating_add(entry_bytes);
        }
        self.effects.mutating = true;
        self.storage = self.storage.insert(key, value);
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<Map<K, V>>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        let change_set = Map::new().diff_change_set(&self.storage).ok();
        let audit = self.audit_trail("MapCollector::finish");
        let mut outcome = CollectOutcome::new(self.storage, audit);
        if let Some(change_set) = change_set.as_ref() {
            outcome = outcome.record_change_set(change_set);
        }
        outcome
    }

    fn iter_error(self, error: IterError) -> Self::Error
    where
        Self: Sized,
    {
        let audit = self.audit_trail("MapCollector::iter_error");
        CollectError::new(
            CollectErrorKind::IteratorFailure,
            "iterator source reported an error during MapCollector::collect",
            audit,
        )
        .with_detail(format!("{error:?}"))
    }
}
