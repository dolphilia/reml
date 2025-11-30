//! `SetCollector` と永続 `Set` の連携実装。

use std::{fmt::Debug, mem};

use serde::Serialize;

use super::super::iter::{EffectLabels, IterError};
use super::{
    CollectError, CollectErrorKind, CollectOutcome, Collector, CollectorAuditTrail,
    CollectorEffectMarkers, CollectorKind, CollectorStageProfile,
};
use crate::collections::persistent::btree::PersistentSet;

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

/// Collector から公開する `Set` 型。
pub type Set<T> = PersistentSet<T>;

pub struct SetCollector<T> {
    storage: Set<T>,
    stage_profile: CollectorStageProfile,
    effects: EffectLabels,
    markers: CollectorEffectMarkers,
}

impl<T: Ord> SetCollector<T> {
    fn audit_trail(&self, source: &'static str) -> CollectorAuditTrail {
        CollectorAuditTrail::new(
            CollectorKind::Set,
            self.stage_profile.snapshot(source),
            self.effects,
            self.markers,
        )
    }

    fn duplicate_error(&self, value: &T) -> CollectError
    where
        T: Debug,
    {
        CollectError::new(
            CollectErrorKind::DuplicateKey,
            format!("duplicate element: {value:?}"),
            self.audit_trail("SetCollector::push"),
        )
        .with_error_key(format!("{value:?}"))
    }
}

impl<T> Collector<T, CollectOutcome<Set<T>>> for SetCollector<T>
where
    T: Ord + Clone + Debug + Serialize,
{
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            storage: Set::new(),
            stage_profile: CollectorStageProfile::for_kind(CollectorKind::Set),
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

    fn push(&mut self, value: T) -> Result<(), Self::Error> {
        if self.storage.contains(&value) {
            return Err(self.duplicate_error(&value));
        }
        let entry_bytes = mem::size_of::<T>();
        self.effects.mutating = true;
        self.effects.mem = true;
        if entry_bytes > 0 {
            self.effects.mem_bytes = self.effects.mem_bytes.saturating_add(entry_bytes);
        }
        self.storage = self.storage.insert(value);
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<Set<T>>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        let change_set = Set::new().diff_change_set(&self.storage).ok();
        let audit = self.audit_trail("SetCollector::finish");
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
        let audit = self.audit_trail("SetCollector::iter_error");
        CollectError::new(
            CollectErrorKind::IteratorFailure,
            "iterator source reported an error during SetCollector::collect",
            audit,
        )
        .with_detail(format!("{error:?}"))
    }
}
