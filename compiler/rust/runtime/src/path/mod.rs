//! Core.Path 抽象の最小実装。
//! 仕様に沿った Path/PathBuf/PathError を提供し、パス検証や正規化を
//! Rust Runtime から利用できるようにする。

use std::borrow::Cow;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::path::{Component, Path as StdPath, PathBuf as StdPathBuf};

use crate::text::Str;

/// Path 関連 API の結果型。
pub type PathResult<T> = Result<T, PathError>;

/// Core.Path の `PathBuf` に相当する所有型。
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct PathBuf {
    inner: StdPathBuf,
}

/// Core.Path の `Path` 参照型。
#[derive(Copy, Clone, Debug)]
pub struct Path<'a> {
    inner: &'a StdPath,
}

/// パス検証時のエラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathError {
    kind: PathErrorKind,
    message: String,
    invalid_input: Option<String>,
}

/// パスエラーの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathErrorKind {
    Empty,
    NullByte,
    InvalidEncoding,
}

impl PathBuf {
    /// 新しい空 PathBuf を生成する。
    pub fn new() -> Self {
        Self {
            inner: StdPathBuf::new(),
        }
    }

    /// 標準 PathBuf から Core.Path Buf へ変換する。
    pub fn from_std(inner: StdPathBuf) -> Self {
        Self { inner }
    }

    /// 標準 PathBuf を取得する。
    pub fn into_std(self) -> StdPathBuf {
        self.inner
    }

    /// 借用参照を取得する。
    pub fn as_path(&self) -> Path<'_> {
        Path { inner: &self.inner }
    }

    /// 正規化済みの PathBuf を返す。
    pub fn normalize(&self) -> PathBuf {
        PathBuf {
            inner: normalize_components(&self.inner),
        }
    }

    /// このパスが絶対パスかを返す。
    pub fn is_absolute(&self) -> bool {
        self.inner.is_absolute()
    }

    /// セグメントを結合して新しい PathBuf を返す。
    pub fn join(&self, segment: impl AsRef<str>) -> PathResult<PathBuf> {
        let segment_ref = segment.as_ref();
        validate_input(segment_ref)?;
        let mut joined = self.inner.clone();
        if !segment_ref.is_empty() {
            joined.push(segment_ref);
        }
        Ok(PathBuf {
            inner: normalize_components(&joined),
        })
    }

    /// 親ディレクトリを返す。
    pub fn parent(&self) -> Option<PathBuf> {
        self.inner.parent().map(|parent| PathBuf {
            inner: parent.to_path_buf(),
        })
    }

    /// パスを lossless に文字列へ変換する。
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.inner.to_string_lossy()
    }

    /// コンポーネント一覧を UTF-8 文字列で返す。
    pub fn components_as_strings(&self) -> Vec<String> {
        self.inner
            .components()
            .map(|component| match component {
                Component::CurDir => ".".to_string(),
                Component::ParentDir => "..".to_string(),
                _ => component.as_os_str().to_string_lossy().into_owned(),
            })
            .collect()
    }
}

impl<'a> Path<'a> {
    /// 標準 Path を参照で取得する。
    pub fn as_std(&self) -> &'a StdPath {
        self.inner
    }

    /// 絶対パスかを返す。
    pub fn is_absolute(&self) -> bool {
        self.inner.is_absolute()
    }

    /// 文字列表現を取得する。
    pub fn to_string_lossy(&self) -> Cow<'a, str> {
        self.inner.to_string_lossy()
    }
}

impl PathError {
    pub fn new(kind: PathErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            invalid_input: None,
        }
    }

    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.invalid_input = Some(input.into());
        self
    }

    pub fn kind(&self) -> PathErrorKind {
        self.kind
    }
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)?;
        if let Some(ref input) = self.invalid_input {
            write!(f, " ({input})")?;
        }
        Ok(())
    }
}

impl Error for PathError {}

impl<'a> TryFrom<Str<'a>> for PathBuf {
    type Error = PathError;

    fn try_from(value: Str<'a>) -> Result<Self, Self::Error> {
        PathBuf::try_from(value.as_str())
    }
}

impl TryFrom<&str> for PathBuf {
    type Error = PathError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        validate_input(value)?;
        Ok(PathBuf {
            inner: StdPathBuf::from(value),
        })
    }
}

impl From<PathBuf> for StdPathBuf {
    fn from(value: PathBuf) -> Self {
        value.into_std()
    }
}

/// `path(str)` に該当するエントリポイント。
pub fn path(text: Str<'_>) -> PathResult<PathBuf> {
    PathBuf::try_from(text)
}

/// `join(base, segment)` のヘルパ。
pub fn join(base: &PathBuf, segment: impl AsRef<str>) -> PathResult<PathBuf> {
    base.join(segment)
}

/// `normalize(path)` のヘルパ。
pub fn normalize(path: &PathBuf) -> PathBuf {
    path.normalize()
}

/// `parent(path)` のヘルパ。
pub fn parent(path: &PathBuf) -> Option<PathBuf> {
    path.parent()
}

/// `is_absolute(path)` のヘルパ。
pub fn is_absolute(path: &PathBuf) -> bool {
    path.is_absolute()
}

/// `components(path)` のヘルパ。
pub fn components(path: &PathBuf) -> Vec<String> {
    path.components_as_strings()
}

fn validate_input(value: &str) -> PathResult<()> {
    if value.is_empty() {
        return Err(PathError::new(
            PathErrorKind::Empty,
            "path must not be empty",
        ));
    }
    if value.as_bytes().iter().any(|b| *b == 0) {
        return Err(
            PathError::new(PathErrorKind::NullByte, "path contains null byte")
                .with_input(value),
        );
    }
    Ok(())
}

fn normalize_components(path: &StdPath) -> StdPathBuf {
    let mut normalized = StdPathBuf::new();
    let is_absolute = path.is_absolute();
    for component in path.components() {
        match component {
            Component::CurDir => continue,
            Component::ParentDir => {
                if !normalized.pop() {
                    if !is_absolute {
                        normalized.push(component.as_os_str());
                    }
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }

    if normalized.as_os_str().is_empty() {
        if is_absolute {
            normalized.push(std::path::MAIN_SEPARATOR.to_string());
        } else {
            normalized.push(".");
        }
    }

    normalized
}
