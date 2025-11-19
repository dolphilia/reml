//! `MapCollector` の雛形実装。キー重複検出と Stage 記録を担保する。

use std::{collections::BTreeMap, fmt::Debug};

use super::super::iter::EffectLabels;
use super::{
    CollectError, CollectErrorKind, CollectOutcome, Collector, CollectorAuditTrail,
    CollectorEffectMarkers, CollectorKind, CollectorStageProfile,
};

const PURE_EFFECTS: EffectLabels = EffectLabels {
    mem: false,
    mutating: false,
    debug: false,
    async_pending: false,
};

/// 永続 `Map` の雛形。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map<K, V> {
    entries: BTreeMap<K, V>,
}

impl<K, V> Map<K, V> {
    fn from_map(entries: BTreeMap<K, V>) -> Self {
        Self { entries }
    }

    pub fn into_map(self) -> BTreeMap<K, V> {
        self.entries
    }
}

pub struct MapCollector<K, V> {
    storage: BTreeMap<K, V>,
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
    K: Ord + Debug,
{
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            storage: BTreeMap::new(),
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
        if self.storage.contains_key(&value.0) {
            return Err(self.duplicate_error(&value.0));
        }
        self.storage.insert(value.0, value.1);
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<Map<K, V>>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        let audit = self.audit_trail("MapCollector::finish");
        let map = Map::from_map(self.storage);
        CollectOutcome::new(map, audit)
    }
}
