use super::Iter;
use crate::prelude::collectors::{CollectError, CollectOutcome, Collector, CollectorAuditTrail};
use std::marker::PhantomData;

/// `Iter.try_collect` で使う Collector と効果伝播のための橋渡し。
pub(crate) struct CollectorBridge<'a, Item, ItemError, Col, Value>
where
    Col: Collector<Item, CollectOutcome<Value>, Error = CollectError>,
{
    iter: &'a Iter<Result<Item, ItemError>>,
    collector: Col,
    phantom: PhantomData<Value>,
}

impl<'a, Item, ItemError, Col, Value> CollectorBridge<'a, Item, ItemError, Col, Value>
where
    Col: Collector<Item, CollectOutcome<Value>, Error = CollectError>,
{
    /// 橋渡しを初期化する。
    pub fn new(iter: &'a Iter<Result<Item, ItemError>>, collector: Col) -> Self {
        Self {
            iter,
            collector,
            phantom: PhantomData,
        }
    }

    /// Collector へ値を渡す。
    pub fn push(&mut self, value: Item) -> Result<(), CollectError> {
        self.collector.push(value)
    }

    /// Collector を終端処理して値と監査情報を得る。
    pub fn finalize(self) -> (Value, CollectorAuditTrail) {
        let outcome = self.collector.finish();
        let audit = outcome.audit().clone();
        let (value, _) = outcome.into_parts();
        (value, audit)
    }

    /// エラー時の監査情報を `Iter` に伝搬する。
    pub fn record_error(&self, error: &CollectError) {
        self.iter.merge_collector_audit(error.audit());
    }
}
