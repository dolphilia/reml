//! フロントエンド診断モジュールのエントリーポイント。

pub mod effects;
pub mod filter;
pub mod formatter;
pub mod json;
pub mod messages;
pub mod recover;
pub mod unicode;

mod model;

pub use effects::StageAuditPayload;
pub use filter::{apply_experimental_stage_policy, should_downgrade_experimental};
pub use formatter::FormatterContext;
pub use model::*;
pub use recover::{ExpectedToken, ExpectedTokenCollector, ExpectedTokensSummary};
