use std::error::Error as StdError;
use std::fmt;

/// 簡易的な `anyhow::Error` 代替。ネットワーク制限下でも
/// `anyhow::Result` 風の API を利用できるよう最小限の機能のみ提供する。
#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl StdError for Error {}

/// `anyhow::Result` 互換の型エイリアス。
pub type Result<T> = std::result::Result<T, Error>;

/// `anyhow!(..)` 相当のヘルパ。
pub fn anyhow(message: impl Into<String>) -> Error {
    Error::new(message)
}
