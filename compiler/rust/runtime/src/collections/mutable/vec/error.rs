#[cfg(feature = "core_prelude")]
use crate::core_prelude::collectors::{CollectError, CollectErrorKind, CollectorAuditTrail};
#[cfg(not(feature = "core_prelude"))]
use crate::prelude::collectors::{CollectError, CollectErrorKind, CollectorAuditTrail};
use std::collections::TryReserveError;

/// VecCollector の `try_reserve` や `with_capacity` で発生する予約エラーを
/// `CollectError` に変換する。
pub fn map_try_reserve_error(
    audit: CollectorAuditTrail,
    source: &'static str,
    err: TryReserveError,
) -> CollectError {
    let detail = format!("{err:?}");
    let kind = CollectErrorKind::MemoryError;
    CollectError::new(
        kind,
        format!("{source} failed during memory reservation"),
        audit,
    )
    .with_detail(detail)
}
