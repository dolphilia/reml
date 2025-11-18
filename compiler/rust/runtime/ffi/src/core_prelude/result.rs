//! `Result` 型の骨格実装。
//!
//! 仕様: `docs/spec/3-1-core-prelude-iteration.md` §2.1

/// Reml コアプレリュードで利用する Result 互換型。
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Result<T, E> {
    /// 正常系。
    Ok(T),
    /// 異常系。
    Err(E),
}

impl<T, E> Result<T, E> {
    /// 今後 `ensure` 系 API を導入するためのプレースホルダ。
    #[inline]
    pub fn todo_placeholder(&self) {
        let _ = self;
        // 実装は WBS 2.1b で追加する。
    }
}
