use serde_json::{Map as JsonMap, Number, Value};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use super::WatchEvent;

const DEFAULT_EVENT_CAPACITY: usize = 64;

/// 監視イベントを収集し `AuditEnvelope.metadata` へ展開するヘルパ。
#[derive(Debug, Clone)]
pub struct WatcherEventRecorder {
    inner: Arc<WatcherEventRecorderInner>,
}

impl WatcherEventRecorder {
    pub fn new(base_paths: Vec<PathBuf>) -> Self {
        Self {
            inner: Arc::new(WatcherEventRecorderInner {
                base_paths,
                state: Mutex::new(WatcherEventRecorderState::default()),
                capacity: DEFAULT_EVENT_CAPACITY,
            }),
        }
    }

    pub fn record_event(&self, event: &WatchEvent, queue_size: u32, delay_ns: u64) {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("watcher audit state mutex poisoned");
        state.total_events = state.total_events.saturating_add(1);
        state
            .events
            .push_back(WatcherAuditEvent::new(event.clone(), queue_size, delay_ns));
        while state.events.len() > self.inner.capacity {
            state.events.pop_front();
        }
    }

    pub fn snapshot(&self) -> WatcherAuditSnapshot {
        let state = self
            .inner
            .state
            .lock()
            .expect("watcher audit state mutex poisoned");
        WatcherAuditSnapshot {
            base_paths: self.inner.base_paths.clone(),
            total_events: state.total_events,
            recent_events: state.events.iter().cloned().collect(),
        }
    }
}

#[derive(Debug)]
struct WatcherEventRecorderInner {
    base_paths: Vec<PathBuf>,
    state: Mutex<WatcherEventRecorderState>,
    capacity: usize,
}

#[derive(Debug, Default)]
struct WatcherEventRecorderState {
    total_events: u64,
    events: VecDeque<WatcherAuditEvent>,
}

/// `AuditEnvelope.metadata` へ展開可能なスナップショット。
#[derive(Debug, Clone)]
pub struct WatcherAuditSnapshot {
    pub base_paths: Vec<PathBuf>,
    pub total_events: u64,
    pub recent_events: Vec<WatcherAuditEvent>,
}

impl WatcherAuditSnapshot {
    pub fn is_empty(&self) -> bool {
        self.total_events == 0
    }

    pub fn into_metadata(self) -> JsonMap<String, Value> {
        let mut metadata = JsonMap::new();
        let paths: Vec<Value> = self
            .base_paths
            .iter()
            .map(|path| Value::String(path_to_string(path)))
            .collect();
        metadata.insert("io.watch.paths".into(), Value::Array(paths));
        metadata.insert(
            "io.watch.events_total".into(),
            Value::Number(Number::from(self.total_events)),
        );
        let events: Vec<Value> = self
            .recent_events
            .into_iter()
            .map(WatcherAuditEvent::into_value)
            .collect();
        metadata.insert("io.watch.events".into(), Value::Array(events));
        metadata
    }
}

/// 単一の監視イベント。
#[derive(Debug, Clone)]
pub struct WatcherAuditEvent {
    event: WatchEvent,
    queue_size: u32,
    delay_ns: u64,
    timestamp_seconds: u64,
    timestamp_nanos: u32,
}

impl WatcherAuditEvent {
    fn new(event: WatchEvent, queue_size: u32, delay_ns: u64) -> Self {
        let (seconds, nanos) = current_timestamp();
        Self {
            event,
            queue_size,
            delay_ns,
            timestamp_seconds: seconds,
            timestamp_nanos: nanos,
        }
    }

    fn into_value(self) -> Value {
        let mut map = JsonMap::new();
        map.insert("kind".into(), Value::String(self.event.kind().to_string()));
        map.insert(
            "path".into(),
            Value::String(path_to_string(self.event.path())),
        );
        map.insert(
            "queue_size".into(),
            Value::Number(Number::from(self.queue_size as u64)),
        );
        map.insert(
            "delay_ns".into(),
            Value::Number(Number::from(self.delay_ns)),
        );
        let mut timestamp = JsonMap::new();
        timestamp.insert(
            "seconds".into(),
            Value::Number(Number::from(self.timestamp_seconds)),
        );
        timestamp.insert(
            "nanos".into(),
            Value::Number(Number::from(self.timestamp_nanos as u64)),
        );
        map.insert("timestamp".into(), Value::Object(timestamp));
        Value::Object(map)
    }
}

fn current_timestamp() -> (u64, u32) {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => (duration.as_secs(), duration.subsec_nanos()),
        Err(_) => (0, 0),
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
