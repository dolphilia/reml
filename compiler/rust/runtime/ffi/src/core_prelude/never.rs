//! `Never` (発散) 型の骨格。
//!
//! 仕様: `docs/spec/3-1-core-prelude-iteration.md` §2.1

/// 値を生成しないための Zero-Sized Type。
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Never {}

impl Never {
    /// `match` 展開での発散伝播を担保するためのヘルパ。
    pub fn absurd(self) -> ! {
        match self {}
    }
}
