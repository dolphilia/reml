use super::Iter;
use crate::prelude::collectors::{CollectError, Collector, CollectorAuditTrail};

/// `Iter.try_collect` で使う Collector と効果伝播のための橋渡し。
pub(crate) struct CollectorBridge<'a, Item, ItemError, Col, Output>
where
    Col: Collector<Item, Output>,
{
    iter: &'a Iter<Result<Item, ItemError>>,
    collector: Col,
}

impl<'a, Item, ItemError, Col, Output> CollectorBridge<'a, Item, ItemError, Col, Output>
where
    Col: Collector<Item, Output>,
{
    /// 橋渡しを初期化する。
    pub fn new(iter: &'a Iter<Result<Item, ItemError>>, collector: Col) -> Self {
        Self { iter, collector }
    }

    /// Collector へ値を渡す。
    pub fn push(&mut self, value: Item) -> Result<(), Col::Error> {
        self.collector.push(value)
    }

    /// Collector を終端処理して値と監査情報を得る。
    pub fn finalize(self) -> (Output, CollectorAuditTrail) {
        self.collector.finish().into_parts()
    }

    /// エラー時の監査情報を `Iter` に伝搬する。
    pub fn record_error(&self, error: &CollectError) {
        self.iter.merge_collector_audit(error.audit());
    }
}
