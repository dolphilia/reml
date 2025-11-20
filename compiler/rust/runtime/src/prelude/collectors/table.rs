//! `TableCollector` の実装。挿入順と重複検出を保証する。

use std::{collections::BTreeSet, fmt::Debug};

use super::super::iter::EffectLabels;
use super::{
    CollectError, CollectErrorKind, CollectOutcome, Collector, CollectorAuditTrail,
    CollectorEffectMarkers, CollectorKind, CollectorStageProfile,
};

const MUTATING_EFFECTS: EffectLabels = EffectLabels {
    mem: false,
    mutating: true,
    debug: false,
    async_pending: false,
};

/// 挿入順を保持する簡易 `Table` 型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table<K, V> {
    entries: Vec<(K, V)>,
}

impl<K, V> Table<K, V> {
    fn from_entries(entries: Vec<(K, V)>) -> Self {
        Self { entries }
    }

    pub fn into_entries(self) -> Vec<(K, V)> {
        self.entries
    }
}

pub struct TableCollector<K, V> {
    entries: Vec<(K, V)>,
    seen: BTreeSet<K>,
    stage_profile: CollectorStageProfile,
    effects: EffectLabels,
    markers: CollectorEffectMarkers,
}

impl<K: Ord, V> TableCollector<K, V> {
    fn audit_trail(&self, source: &'static str) -> CollectorAuditTrail {
        CollectorAuditTrail::new(
            CollectorKind::Table,
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
            self.audit_trail("TableCollector::push"),
        )
        .with_error_key(format!("{key:?}"))
    }
}

impl<K, V> Collector<(K, V), CollectOutcome<Table<K, V>>> for TableCollector<K, V>
where
    K: Ord + Debug + Clone,
{
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            entries: Vec::new(),
            seen: BTreeSet::new(),
            stage_profile: CollectorStageProfile::for_kind(CollectorKind::Table),
            effects: MUTATING_EFFECTS,
            markers: CollectorEffectMarkers::default(),
        }
    }

    fn with_capacity(capacity: usize) -> Self
    where
        Self: Sized,
    {
        let mut collector = Self::new();
        collector.entries.reserve(capacity);
        collector
    }

    fn push(&mut self, value: (K, V)) -> Result<(), Self::Error> {
        if !self.seen.insert(value.0.clone()) {
            return Err(self.duplicate_error(&value.0));
        }
        self.entries.push(value);
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<Table<K, V>>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        let audit = self.audit_trail("TableCollector::finish");
        let table = Table::from_entries(self.entries);
        CollectOutcome::new(table, audit)
    }
}
