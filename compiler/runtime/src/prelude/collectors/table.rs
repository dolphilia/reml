//! `TableCollector` の実装。挿入順と重複検出を保証する。

use std::{fmt::Debug, hash::Hash, mem};

use super::super::iter::{EffectLabels, IterError};
use super::{
    CollectError, CollectErrorKind, CollectOutcome, Collector, CollectorAuditTrail,
    CollectorEffectMarkers, CollectorKind, CollectorStageProfile,
};
pub use crate::collections::mutable::Table;

const TABLE_EFFECTS: EffectLabels = EffectLabels {
    mem: false,
    mutating: true,
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

pub struct TableCollector<K, V>
where
    K: Eq + Hash + Clone,
{
    table: Table<K, V>,
    stage_profile: CollectorStageProfile,
    effects: EffectLabels,
    markers: CollectorEffectMarkers,
}

impl<K, V> TableCollector<K, V>
where
    K: Eq + Hash + Clone + Debug,
{
    fn audit_trail(&self, source: &'static str) -> CollectorAuditTrail {
        CollectorAuditTrail::new(
            CollectorKind::Table,
            self.stage_profile.snapshot(source),
            self.effects,
            self.markers,
        )
    }

    fn duplicate_error(&self, key: &K) -> CollectError {
        CollectError::new(
            CollectErrorKind::DuplicateKey,
            format!("duplicate key: {key:?}"),
            self.audit_trail("TableCollector::push"),
        )
        .with_error_key(format!("{key:?}"))
    }
}

impl<K, V> Collector<(K, V), CollectOutcome<Table<K, V>>> for TableCollector<K, V>
where
    K: Eq + Hash + Clone + Debug,
{
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            table: Table::new(),
            stage_profile: CollectorStageProfile::for_kind(CollectorKind::Table),
            effects: TABLE_EFFECTS,
            markers: CollectorEffectMarkers::default(),
        }
    }

    fn with_capacity(capacity: usize) -> Self
    where
        Self: Sized,
    {
        let collector = Self::new();
        let _ = capacity;
        collector
    }

    fn push(&mut self, value: (K, V)) -> Result<(), Self::Error> {
        if self.table.contains_key(&value.0) {
            return Err(self.duplicate_error(&value.0));
        }
        let entry_bytes = mem::size_of::<K>() + mem::size_of::<V>();
        self.effects.mem = true;
        self.effects.mem_bytes = self.effects.mem_bytes.saturating_add(entry_bytes);
        self.table.insert(value.0, value.1);
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<Table<K, V>>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        let audit = self.audit_trail("TableCollector::finish");
        CollectOutcome::new(self.table, audit)
    }

    fn iter_error(self, error: IterError) -> Self::Error
    where
        Self: Sized,
    {
        let audit = self.audit_trail("TableCollector::iter_error");
        CollectError::new(
            CollectErrorKind::IteratorFailure,
            "iterator source reported an error during TableCollector::collect",
            audit,
        )
        .with_detail(format!("{error:?}"))
    }
}
