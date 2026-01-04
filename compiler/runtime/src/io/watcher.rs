use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

use glob::Pattern;
use notify::event::{CreateKind, EventKind, ModifyKind, RemoveKind};
use notify::{
    Config as NotifyConfig, Event, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher as _,
};

use super::{
    adapters::{
        WatcherAdapter, CAP_FS_WATCH_NATIVE, CAP_FS_WATCH_RECURSIVE, CAP_WATCH_RESOURCE_LIMITS,
    },
    effects::{
        record_async_io_operation, record_watch_metrics, take_io_effects_snapshot,
        take_watch_metrics_snapshot, WatchMetricsSnapshot,
    },
    watcher_audit::{WatcherAuditSnapshot, WatcherEventRecorder},
    IoContext, IoError, IoErrorKind, IoResult,
};

/// ファイルシステムイベントを通知するハンドル。
#[derive(Debug)]
pub struct Watcher {
    handle: WatcherHandle,
}

impl Watcher {
    fn new(state: Arc<WatcherState>) -> Self {
        Self {
            handle: WatcherHandle { state },
        }
    }

    /// 監視を終了する。
    pub fn close(self) -> IoResult<()> {
        self.handle.close()
    }

    pub fn handle(&self) -> WatcherHandle {
        self.handle.clone()
    }

    /// 収集済み監視イベントのスナップショットを取得する。
    pub fn audit_snapshot(&self) -> WatcherAuditSnapshot {
        self.handle.audit_snapshot()
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = self.handle.close();
    }
}

/// 監視ハンドルを複製するための参照型。
#[derive(Debug, Clone)]
pub struct WatcherHandle {
    state: Arc<WatcherState>,
}

impl WatcherHandle {
    pub fn close(&self) -> IoResult<()> {
        self.state.close()
    }

    pub fn audit_snapshot(&self) -> WatcherAuditSnapshot {
        self.state.audit_snapshot()
    }
}

/// ファイル監視イベント。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

impl WatchEvent {
    pub fn path(&self) -> &Path {
        match self {
            WatchEvent::Created(path) | WatchEvent::Modified(path) | WatchEvent::Deleted(path) => {
                path.as_path()
            }
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            WatchEvent::Created(_) => "created",
            WatchEvent::Modified(_) => "modified",
            WatchEvent::Deleted(_) => "deleted",
        }
    }
}

/// 監視リソースの消費を抑制するパラメータ。
#[derive(Debug, Clone)]
pub struct WatchLimits {
    pub max_events_per_second: Option<u32>,
    pub max_depth: Option<u8>,
    pub exclude_patterns: Vec<String>,
}

impl Default for WatchLimits {
    fn default() -> Self {
        Self {
            max_events_per_second: None,
            max_depth: None,
            exclude_patterns: Vec::new(),
        }
    }
}

impl WatchLimits {
    pub fn without_limits() -> Self {
        Self::default()
    }

    pub fn uses_resource_limits(&self) -> bool {
        self.max_events_per_second.is_some()
            || self.max_depth.is_some()
            || !self.exclude_patterns.is_empty()
    }
}

/// `watch` API のエントリポイント。
pub fn watch<I, P, F>(paths: I, callback: F) -> IoResult<Watcher>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
    F: Fn(WatchEvent) + Send + Sync + 'static,
{
    watch_with_limits(paths, WatchLimits::default(), callback)
}

/// 制限付き `watch`。
pub fn watch_with_limits<I, P, F>(paths: I, limits: WatchLimits, callback: F) -> IoResult<Watcher>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
    F: Fn(WatchEvent) + Send + Sync + 'static,
{
    let resolved_paths: Vec<PathBuf> = paths
        .into_iter()
        .map(|p| {
            let path = p.as_ref();
            fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
        })
        .collect();
    if resolved_paths.is_empty() {
        return Err(invalid_input_error(
            "watch requires at least one path",
            None,
        ));
    }
    for path in &resolved_paths {
        if !path.exists() {
            return Err(invalid_input_error(
                "watch path does not exist",
                Some(path.clone()),
            ));
        }
    }

    let adapter = WatcherAdapter::global();
    let mut capability_ctx = watch_context("watch", None, CAP_FS_WATCH_NATIVE);
    adapter
        .ensure_native_capability()
        .map_err(|err| err.with_context(capability_ctx.clone()))?;
    ensure_watcher_feature(WatchFeature::FsChange, &capability_ctx)?;

    if requires_recursive(&limits) {
        capability_ctx = capability_ctx.with_capability(CAP_FS_WATCH_RECURSIVE);
        adapter
            .ensure_recursive_capability()
            .map_err(|err| err.with_context(capability_ctx.clone()))?;
        ensure_watcher_feature(WatchFeature::Recursive, &capability_ctx)?;
    }

    if limits.uses_resource_limits() {
        let resource_ctx = watch_context("watch.limits", None, CAP_WATCH_RESOURCE_LIMITS);
        adapter
            .ensure_resource_limit_capability()
            .map_err(|err| err.with_context(resource_ctx.clone()))?;
        ensure_watcher_feature(WatchFeature::ResourceLimits, &resource_ctx)?;
    }

    record_async_io_operation();
    let mut base_context = watch_context("watch", None, CAP_FS_WATCH_NATIVE);
    base_context.set_effects(take_io_effects_snapshot());
    base_context.set_watch_stats_from_metrics(take_watch_metrics_snapshot());

    let exclude =
        build_exclude_set(&limits).map_err(|err| err.with_context(base_context.clone()))?;

    let comparison_paths: Vec<PathBuf> = resolved_paths
        .iter()
        .map(|path| fs::canonicalize(path).unwrap_or_else(|_| path.clone()))
        .collect();
    let config = WatchRuntimeConfig {
        comparison_paths,
        limits: limits.clone(),
        exclude,
    };

    let callback = Arc::new(callback);
    let (event_tx, event_rx) = mpsc::channel();
    let queue_depth = Arc::new(AtomicUsize::new(0));
    let sender_depth = queue_depth.clone();
    let use_poll_backend = std::env::var("REML_WATCHER_BACKEND")
        .map(|value| value.eq_ignore_ascii_case("poll"))
        .unwrap_or(false);
    let notify_config = if use_poll_backend {
        NotifyConfig::default().with_poll_interval(Duration::from_millis(100))
    } else {
        NotifyConfig::default()
    };
    let make_event_handler =
        |event_tx: Sender<ScheduledEvent>, sender_depth: Arc<AtomicUsize>| {
            move |res: notify::Result<Event>| {
                sender_depth.fetch_add(1, Ordering::SeqCst);
                let timestamp = Instant::now();
                if event_tx.send((timestamp, res)).is_err() {
                    sender_depth.fetch_sub(1, Ordering::SeqCst);
                }
            }
        };
    let mut watcher: Box<dyn notify::Watcher + Send> = if use_poll_backend {
        Box::new(
            PollWatcher::new(
                make_event_handler(event_tx.clone(), Arc::clone(&sender_depth)),
                notify_config,
            )
            .map_err(|err| {
                notify_to_io_error(err, None, base_context.clone(), WatchMetricsSnapshot::EMPTY)
            })?,
        )
    } else {
        Box::new(
            RecommendedWatcher::new(
                make_event_handler(event_tx, sender_depth),
                notify_config,
            )
            .map_err(|err| {
                notify_to_io_error(err, None, base_context.clone(), WatchMetricsSnapshot::EMPTY)
            })?,
        )
    };

    let recursive_mode = recursive_mode(&limits);
    for path in &resolved_paths {
        watcher.watch(path, recursive_mode).map_err(|err| {
            notify_to_io_error(
                err,
                Some(path),
                base_context.clone(),
                WatchMetricsSnapshot::EMPTY,
            )
        })?;
    }

    let audit_recorder = WatcherEventRecorder::new(resolved_paths.clone());
    let (command_tx, command_rx) = mpsc::channel();
    let state = Arc::new(WatcherState::new(command_tx, audit_recorder.clone()));
    let runtime = WatchRuntime {
        config,
        callback,
        event_rx,
        command_rx,
        queue_depth,
        error_state: Arc::clone(&state.error),
        base_context: base_context.clone(),
        audit: audit_recorder,
        watcher,
    };

    let join_handle = thread::Builder::new()
        .name("core-io-watcher".into())
        .spawn(move || run_watcher(runtime))
        .map_err(|err| {
            IoError::new(
                IoErrorKind::OutOfMemory,
                format!("failed to spawn watcher thread: {err}"),
            )
        })?;

    state.join_handle.lock().unwrap().replace(join_handle);
    Ok(Watcher::new(state))
}

/// `Watcher` を明示的にクローズするユーティリティ。
pub fn close_watcher(watcher: Watcher) -> IoResult<()> {
    watcher.close()
}

fn run_watcher(runtime: WatchRuntime) {
    let _watcher = &runtime.watcher;
    let mut rate_limiter = RateLimiter::new(runtime.config.limits.max_events_per_second);
    loop {
        if runtime.should_stop() {
            break;
        }
        match runtime.event_rx.recv_timeout(Duration::from_millis(100)) {
            Ok((started, result)) => {
                runtime.queue_depth.fetch_sub(1, Ordering::SeqCst);
                match result {
                    Ok(event) => runtime.handle_event(event, started, &mut rate_limiter),
                    Err(err) => {
                        let delay_ns = duration_since(started, Instant::now());
                        let metrics = WatchMetricsSnapshot::new(
                            runtime.queue_depth.load(Ordering::SeqCst),
                            delay_ns,
                        );
                        let io_error =
                            notify_to_io_error(err, None, runtime.base_context.clone(), metrics);
                        runtime.set_error(io_error);
                        break;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

struct WatchRuntime {
    config: WatchRuntimeConfig,
    callback: Arc<dyn Fn(WatchEvent) + Send + Sync + 'static>,
    event_rx: Receiver<ScheduledEvent>,
    command_rx: Receiver<WatcherCommand>,
    queue_depth: Arc<AtomicUsize>,
    error_state: Arc<Mutex<Option<IoError>>>,
    base_context: IoContext,
    audit: WatcherEventRecorder,
    watcher: Box<dyn notify::Watcher + Send>,
}

impl WatchRuntime {
    fn should_stop(&self) -> bool {
        match self.command_rx.try_recv() {
            Ok(WatcherCommand::Shutdown) => true,
            Err(TryRecvError::Disconnected) => true,
            Err(TryRecvError::Empty) => false,
        }
    }

    fn handle_event(&self, event: Event, started: Instant, rate_limiter: &mut RateLimiter) {
        let now = Instant::now();
        let Event { kind, paths, .. } = event;
        let mut delivered_events: Vec<WatchEvent> = Vec::new();
        for path in paths {
            if !self.config.is_allowed(&path) {
                continue;
            }
            if let Some(watch_event) = WatchEvent::from_kind(kind.clone(), path.clone()) {
                if !rate_limiter.allow(now) {
                    continue;
                }
                (self.callback)(watch_event.clone());
                delivered_events.push(watch_event);
            }
        }

        let queue_size = self.queue_depth.load(Ordering::SeqCst);
        let delay_ns = duration_since(started, now);
        record_async_io_operation();
        record_watch_metrics(queue_size, delay_ns);
        let _ = take_watch_metrics_snapshot();
        for event in delivered_events {
            self.audit
                .record_event(&event, queue_size.min(u32::MAX as usize) as u32, delay_ns);
        }
    }

    fn set_error(&self, error: IoError) {
        let mut guard = self
            .error_state
            .lock()
            .expect("watcher error mutex poisoned");
        guard.replace(error);
    }
}

impl WatchEvent {
    fn from_kind(kind: EventKind, path: PathBuf) -> Option<Self> {
        match kind {
            EventKind::Create(CreateKind::File)
            | EventKind::Create(CreateKind::Any)
            | EventKind::Create(CreateKind::Folder) => Some(WatchEvent::Created(path)),
            EventKind::Create(_) => Some(WatchEvent::Created(path)),
            EventKind::Modify(ModifyKind::Name(_))
            | EventKind::Modify(ModifyKind::Any)
            | EventKind::Modify(ModifyKind::Data(_))
            | EventKind::Modify(ModifyKind::Metadata(_)) => Some(WatchEvent::Modified(path)),
            EventKind::Modify(_) => Some(WatchEvent::Modified(path)),
            EventKind::Remove(RemoveKind::File)
            | EventKind::Remove(RemoveKind::Any)
            | EventKind::Remove(RemoveKind::Folder) => Some(WatchEvent::Deleted(path)),
            EventKind::Remove(_) => Some(WatchEvent::Deleted(path)),
            EventKind::Access(_) => None,
            EventKind::Other => None,
            EventKind::Any => Some(WatchEvent::Modified(path)),
        }
    }
}

type ScheduledEvent = (Instant, notify::Result<Event>);

#[derive(Debug)]
struct WatcherState {
    command_tx: Mutex<Option<Sender<WatcherCommand>>>,
    join_handle: Mutex<Option<thread::JoinHandle<()>>>,
    error: Arc<Mutex<Option<IoError>>>,
    audit: WatcherEventRecorder,
}

impl WatcherState {
    fn new(command_tx: Sender<WatcherCommand>, audit: WatcherEventRecorder) -> Self {
        Self {
            command_tx: Mutex::new(Some(command_tx)),
            join_handle: Mutex::new(None),
            error: Arc::new(Mutex::new(None)),
            audit,
        }
    }

    fn close(&self) -> IoResult<()> {
        {
            let mut guard = self
                .command_tx
                .lock()
                .expect("watcher command mutex poisoned");
            if let Some(tx) = guard.take() {
                let _ = tx.send(WatcherCommand::Shutdown);
            }
        }
        if let Some(handle) = self
            .join_handle
            .lock()
            .expect("watcher handle mutex poisoned")
            .take()
        {
            handle
                .join()
                .map_err(|_| IoError::new(IoErrorKind::UnexpectedEof, "watcher thread panicked"))?;
        }
        if let Some(err) = self
            .error
            .lock()
            .expect("watcher error mutex poisoned")
            .take()
        {
            return Err(err);
        }
        Ok(())
    }

    fn audit_snapshot(&self) -> WatcherAuditSnapshot {
        self.audit.snapshot()
    }
}

struct WatchRuntimeConfig {
    comparison_paths: Vec<PathBuf>,
    limits: WatchLimits,
    exclude: Option<Vec<Pattern>>,
}

impl WatchRuntimeConfig {
    fn is_allowed(&self, path: &Path) -> bool {
        self.within_depth(path) && !self.matches_exclude(path)
    }

    fn within_depth(&self, path: &Path) -> bool {
        let candidate = normalize_event_path(path);
        match self.limits.max_depth {
            None => true,
            Some(limit) => self.comparison_paths.iter().any(|base| {
                if base == &candidate {
                    return true;
                }
                candidate
                    .strip_prefix(base)
                    .ok()
                    .map(|relative| relative.components().count() <= limit as usize)
                    .unwrap_or(false)
            }),
        }
    }

    fn matches_exclude(&self, path: &Path) -> bool {
        self.exclude
            .as_ref()
            .map(|patterns| patterns.iter().any(|pattern| pattern.matches_path(path)))
            .unwrap_or(false)
    }
}

struct RateLimiter {
    limit: Option<u32>,
    window: VecDeque<Instant>,
}

fn normalize_event_path(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        return canonical;
    }
    if let Some(parent) = path.parent() {
        if let Ok(canonical_parent) = fs::canonicalize(parent) {
            if let Some(name) = path.file_name() {
                return canonical_parent.join(name);
            }
        }
    }
    path.to_path_buf()
}

impl RateLimiter {
    fn new(limit: Option<u32>) -> Self {
        Self {
            limit: limit.filter(|value| *value > 0),
            window: VecDeque::new(),
        }
    }

    fn allow(&mut self, now: Instant) -> bool {
        if let Some(limit) = self.limit {
            while let Some(front) = self.window.front().copied() {
                if now.duration_since(front) > Duration::from_secs(1) {
                    self.window.pop_front();
                } else {
                    break;
                }
            }
            if self.window.len() as u32 >= limit {
                return false;
            }
            self.window.push_back(now);
        }
        true
    }
}

#[derive(Debug)]
enum WatcherCommand {
    Shutdown,
}

fn requires_recursive(limits: &WatchLimits) -> bool {
    match limits.max_depth {
        None => true,
        Some(depth) => depth > 0,
    }
}

fn recursive_mode(limits: &WatchLimits) -> RecursiveMode {
    if requires_recursive(limits) {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    }
}

fn build_exclude_set(limits: &WatchLimits) -> IoResult<Option<Vec<Pattern>>> {
    if limits.exclude_patterns.is_empty() {
        return Ok(None);
    }
    let mut patterns = Vec::with_capacity(limits.exclude_patterns.len());
    for pattern in &limits.exclude_patterns {
        let compiled = Pattern::new(pattern).map_err(|err| {
            invalid_input_error(
                format!("invalid watch exclude pattern `{pattern}`: {err}"),
                None,
            )
        })?;
        patterns.push(compiled);
    }
    Ok(Some(patterns))
}

fn watch_context(
    operation: &'static str,
    path: Option<PathBuf>,
    capability: &'static str,
) -> IoContext {
    let mut context = IoContext::new(operation).with_capability(capability);
    if let Some(path_buf) = path {
        context = context.with_path(path_buf);
    }
    context
}

fn notify_to_io_error(
    err: notify::Error,
    path: Option<&Path>,
    mut context: IoContext,
    metrics: WatchMetricsSnapshot,
) -> IoError {
    context.set_watch_stats_from_metrics(metrics);
    let mut error = IoError::new(IoErrorKind::InvalidInput, err.to_string());
    if let Some(path) = path {
        error = error.with_path(path.to_path_buf());
    }
    error.with_context(context)
}

fn invalid_input_error(message: impl Into<String>, path: Option<PathBuf>) -> IoError {
    let mut error = IoError::new(IoErrorKind::InvalidInput, message);
    if let Some(path_buf) = path {
        error = error.with_path(path_buf);
    }
    error
}

fn duration_since(instant: Instant, now: Instant) -> u64 {
    now.checked_duration_since(instant)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug)]
enum WatchFeature {
    FsChange,
    Recursive,
    ResourceLimits,
}

impl WatchFeature {
    fn id(&self) -> &'static str {
        match self {
            WatchFeature::FsChange => "watcher.fschange",
            WatchFeature::Recursive => "watcher.recursive",
            WatchFeature::ResourceLimits => "watcher.resource_limits",
        }
    }

    fn is_supported(&self) -> bool {
        cfg!(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows"
        ))
    }
}

fn ensure_watcher_feature(feature: WatchFeature, context: &IoContext) -> IoResult<()> {
    if feature.is_supported() {
        Ok(())
    } else {
        Err(unsupported_platform_error(feature, context.clone()))
    }
}

fn unsupported_platform_error(feature: WatchFeature, context: IoContext) -> IoError {
    let platform = std::env::consts::OS;
    IoError::new(
        IoErrorKind::UnsupportedPlatform,
        format!(
            "watcher feature `{}` is not available on platform `{platform}`",
            feature.id()
        ),
    )
    .with_context(context)
    .with_platform(platform)
    .with_feature(feature.id())
}
