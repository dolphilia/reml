//! `SetCollector` の雛形実装。重複検出と Stage 監査をシンプルにまとめる。

use std::{collections::BTreeSet, fmt::Debug};

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

/// `Set` の最小型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set<T> {
    entries: BTreeSet<T>,
}

impl<T> Set<T> {
    fn from_set(entries: BTreeSet<T>) -> Self {
        Self { entries }
    }

    pub fn into_set(self) -> BTreeSet<T> {
        self.entries
    }
}

pub struct SetCollector<T> {
    storage: BTreeSet<T>,
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
    T: Ord + Debug + Clone,
{
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            storage: BTreeSet::new(),
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
        if !self.storage.insert(value.clone()) {
            return Err(self.duplicate_error(&value));
        }
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<Set<T>>
    where
        Self: Sized,
    {
        self.markers.record_finish();
        let audit = self.audit_trail("SetCollector::finish");
        let set = Set::from_set(self.storage);
        CollectOutcome::new(set, audit)
    }
}
