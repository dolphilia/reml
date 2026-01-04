//! 可変コレクション群 (`Vec`/`Cell`/`Ref`/`Table`)。
//! 仕様ドラフトに沿った API を Rust 実装で提供する。

pub mod cell;
pub mod r#ref;
pub mod table;
pub mod vec;

pub use cell::{Cell, EffectfulCell};
pub use r#ref::{BorrowError, EffectfulRef, Ref, RefGuard, RefMutGuard};
pub use table::{EffectfulTable, Table, TableIter};
pub use vec::{CoreVec, EffectfulVec};
