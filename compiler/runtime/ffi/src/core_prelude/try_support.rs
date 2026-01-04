//! `Try` トレイトの骨格。
//!
//! `core::ops::Try` とは独立した Reml 独自契約を想定しており、WBS 2.1b で
//! `Option`/`Result` との連携を埋め込む。

/// Reml の `try` 記法に対応するコントラクト。
pub trait Try {
    /// 正常時に得られる値。
    type Output;
    /// エラー時に保持する Residual。
    type Residual;

    /// 成功値から `Try` を構築する。
    fn from_output(output: Self::Output) -> Self;

    /// `Self` から Residual へ遷移する。
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output>;
}

/// `std::ops::ControlFlow` に似た分岐結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlFlow<B, C> {
    /// 失敗（Break）ケース。
    Break(B),
    /// 成功（Continue）ケース。
    Continue(C),
}

/// `Try` 実行時に添付する追加メタデータ。
#[derive(Default, Debug)]
pub struct TryContext {
    /// 仕様リンクや診断キーの記録先。
    pub provenance: Option<&'static str>,
}
