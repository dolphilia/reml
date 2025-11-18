//! `Option` 型の骨格実装。
//!
//! 仕様: `docs/spec/3-1-core-prelude-iteration.md` §2.1

/// Reml コアプレリュードで利用する Option 互換型。
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Option<T> {
    /// 値を保持するケース。
    Some(T),
    /// 値が存在しないケース。
    None,
}

impl<T> Option<T> {
    /// 将来の API 実装で `effect {debug}` を付与する対象をまとめるためのスタブ。
    #[inline]
    pub fn todo_placeholder(&self) {
        let _ = self;
        // 実装は WBS 2.1b で追加する。
    }
}
