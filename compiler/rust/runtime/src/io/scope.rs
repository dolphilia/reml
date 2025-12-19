use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};

use super::{
    adapters::{FsAdapter, CAP_IO_FS_WRITE},
    effects::{blocking_io_effect_labels, record_io_operation},
    options::FileOptions,
    File, IoContext, IoError, IoResult,
};

/// スコープ終了時に任意のクリーンアップを実行する汎用ガード。
#[allow(dead_code)]
#[derive(Debug)]
pub struct ScopeGuard<T, F>
where
    F: FnOnce(T),
{
    value: Option<T>,
    on_drop: Option<F>,
}

impl<T, F> ScopeGuard<T, F>
where
    F: FnOnce(T),
{
    pub fn new(value: T, on_drop: F) -> Self {
        Self {
            value: Some(value),
            on_drop: Some(on_drop),
        }
    }

    pub fn value(&self) -> &T {
        self.value.as_ref().expect("scope guard released value")
    }

    pub fn value_mut(&mut self) -> &mut T {
        self.value.as_mut().expect("scope guard released value")
    }

    pub fn into_inner(mut self) -> T {
        self.on_drop.take();
        self.value
            .take()
            .expect("scope guard released value before into_inner")
    }
}

impl<T, F> Drop for ScopeGuard<T, F>
where
    F: FnOnce(T),
{
    fn drop(&mut self) {
        if let (Some(value), Some(on_drop)) = (self.value.take(), self.on_drop.take()) {
            on_drop(value);
        }
    }
}

/// File API 用のスコープ種別。
#[derive(Debug, Clone)]
pub enum ScopedFileMode {
    Open,
    Create(FileOptions),
}

impl Default for ScopedFileMode {
    fn default() -> Self {
        Self::Open
    }
}

impl ScopedFileMode {
    pub fn create(options: FileOptions) -> Self {
        Self::Create(options)
    }
}

/// `File` をスコープ内で安全に扱うヘルパ。
pub fn with_file<P, F, T>(path: P, mode: ScopedFileMode, f: F) -> IoResult<T>
where
    P: AsRef<Path>,
    F: FnOnce(&mut File) -> IoResult<T>,
{
    let path_ref = path.as_ref();
    let mut file = match mode {
        ScopedFileMode::Open => File::open(path_ref)?,
        ScopedFileMode::Create(options) => File::create(path_ref, options)?,
    };
    f(&mut file)
}

/// 一時ディレクトリを作成し、スコープ終了時に削除する。
pub fn with_temp_dir<F, T>(prefix: impl AsRef<str>, f: F) -> IoResult<T>
where
    F: FnOnce(&TempDirGuard) -> IoResult<T>,
{
    let adapter = FsAdapter::global();

    let path = unique_temp_dir(prefix.as_ref());
    let context = temp_dir_context("temp_dir.create", &path);
    adapter
        .ensure_write_capability()
        .map_err(|err| err.with_context(context.clone()))?;
    record_io_operation(1);
    fs::create_dir_all(&path).map_err(|err| IoError::from_std(err, context))?;

    let guard = TempDirGuard::new(path);
    f(&guard)
}

/// リークトラッカーのスナップショット。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeakTrackerSnapshot {
    pub open_files: usize,
    pub temp_dirs: usize,
}

pub fn leak_tracker_snapshot() -> LeakTrackerSnapshot {
    LeakTrackerSnapshot {
        open_files: FILE_HANDLES.load(Ordering::SeqCst),
        temp_dirs: TEMP_DIRS.load(Ordering::SeqCst),
    }
}

pub fn reset_leak_tracker() {
    FILE_HANDLES.store(0, Ordering::SeqCst);
    TEMP_DIRS.store(0, Ordering::SeqCst);
}

#[derive(Debug)]
pub struct TempDirGuard {
    path: PathBuf,
    #[allow(dead_code)]
    tracker: ResourceGuard,
}

impl TempDirGuard {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            tracker: ResourceGuard::new(ResourceKind::TempDir),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
        // `ResourceGuard` の Drop でカウンタを減算する。
    }
}

#[derive(Debug)]
pub(crate) struct FileHandleGuard {
    #[allow(dead_code)]
    tracker: ResourceGuard,
}

impl FileHandleGuard {
    pub(crate) fn new() -> Self {
        Self {
            tracker: ResourceGuard::new(ResourceKind::FileHandle),
        }
    }
}

#[derive(Debug)]
struct ResourceGuard {
    kind: ResourceKind,
}

impl ResourceGuard {
    fn new(kind: ResourceKind) -> Self {
        increment_counter(kind);
        Self { kind }
    }
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        decrement_counter(self.kind);
    }
}

#[derive(Debug, Clone, Copy)]
enum ResourceKind {
    FileHandle,
    TempDir,
}

fn increment_counter(kind: ResourceKind) {
    match kind {
        ResourceKind::FileHandle => {
            FILE_HANDLES.fetch_add(1, Ordering::SeqCst);
        }
        ResourceKind::TempDir => {
            TEMP_DIRS.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn decrement_counter(kind: ResourceKind) {
    match kind {
        ResourceKind::FileHandle => {
            FILE_HANDLES.fetch_sub(1, Ordering::SeqCst);
        }
        ResourceKind::TempDir => {
            TEMP_DIRS.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

fn temp_dir_context(operation: &'static str, path: &Path) -> IoContext {
    IoContext::new(operation)
        .with_path(path.to_path_buf())
        .with_capability(CAP_IO_FS_WRITE)
        .with_effects(blocking_io_effect_labels())
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let suffix = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut base = env::temp_dir();
    let pid = std::process::id();
    let sanitized = prefix
        .chars()
        .map(|ch| if matches!(ch, '/' | '\\') { '_' } else { ch })
        .collect::<String>();
    base.push(format!("{sanitized}-reml-{pid}-{suffix}"));
    base
}

static FILE_HANDLES: AtomicUsize = AtomicUsize::new(0);
static TEMP_DIRS: AtomicUsize = AtomicUsize::new(0);
