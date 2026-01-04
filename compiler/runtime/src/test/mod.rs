//! Core.Test 仕様の最小実装。
//! スナップショットの保持はプロセス内メモリに限定し、IO 連携は後続フェーズで実装する。

use once_cell::sync::Lazy;
use serde_json::{Map as JsonMap, Value};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::panic::UnwindSafe;
use std::sync::Mutex;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::audit::{AuditEnvelope, AuditEvent, AuditEventKind};
use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic};

const DEFAULT_SNAPSHOT_MAX_BYTES: usize = 1024 * 1024;

pub mod dsl;

type SnapshotStore = HashMap<String, SnapshotEntry>;

static SNAPSHOTS: Lazy<Mutex<SnapshotStore>> = Lazy::new(|| Mutex::new(HashMap::new()));
static TEST_AUDIT_EVENTS: Lazy<Mutex<Vec<AuditEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));
static TEST_DIAGNOSTICS: Lazy<Mutex<Vec<GuardDiagnostic>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// テスト API の結果型。
pub type TestResult = Result<(), TestError>;

/// Core.Test のエラー。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestError {
    pub kind: TestErrorKind,
    pub message: String,
    pub context: BTreeMap<String, String>,
}

impl TestError {
    pub fn new(kind: TestErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: BTreeMap::new(),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    pub fn with_case_name(self, case_name: impl Into<String>) -> Self {
        self.with_context("case_name", case_name)
    }

    pub fn into_diagnostic(&self) -> GuardDiagnostic {
        let mut extensions = JsonMap::new();
        let mut test_payload = JsonMap::new();
        if let Some(case_name) = self.context.get("case_name") {
            test_payload.insert("case_name".into(), Value::String(case_name.clone()));
        }
        if !test_payload.is_empty() {
            extensions.insert("test".into(), Value::Object(test_payload));
        }
        GuardDiagnostic {
            code: "test.failed",
            domain: "test",
            severity: DiagnosticSeverity::Error,
            message: self.message.clone(),
            notes: Vec::new(),
            extensions,
            audit_metadata: JsonMap::new(),
        }
    }
}

/// テスト失敗の種別。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestErrorKind {
    AssertionFailed,
    SnapshotMismatch,
    SnapshotMissing,
    HarnessFailure,
    FuzzCrash,
}

/// スナップショットのポリシー。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotPolicy {
    pub mode: SnapshotMode,
    pub normalize: bool,
    pub max_bytes: usize,
}

impl SnapshotPolicy {
    pub fn verify() -> Self {
        Self {
            mode: SnapshotMode::Verify,
            normalize: true,
            max_bytes: DEFAULT_SNAPSHOT_MAX_BYTES,
        }
    }

    pub fn update() -> Self {
        Self {
            mode: SnapshotMode::Update,
            normalize: true,
            max_bytes: DEFAULT_SNAPSHOT_MAX_BYTES,
        }
    }

    pub fn record() -> Self {
        Self {
            mode: SnapshotMode::Record,
            normalize: true,
            max_bytes: DEFAULT_SNAPSHOT_MAX_BYTES,
        }
    }
}

/// スナップショットの更新モード。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotMode {
    Verify,
    Update,
    Record,
}

/// テストケース。
pub struct TestCase {
    pub name: String,
    pub body: Box<dyn Fn() -> TestResult + Send + Sync>,
}

impl TestCase {
    pub fn new(
        name: impl Into<String>,
        body: impl Fn() -> TestResult + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            body: Box::new(body),
        }
    }
}

/// テーブル駆動テストの入力。
pub struct TableCase<T> {
    pub input: T,
    pub expected: String,
}

/// 値の一致を検証する。
pub fn assert_eq<T: Eq + Debug>(actual: T, expected: T) -> TestResult {
    if actual == expected {
        Ok(())
    } else {
        Err(TestError::new(
            TestErrorKind::AssertionFailed,
            format!("assert_eq failed: actual={actual:?} expected={expected:?}"),
        ))
    }
}

/// 既定ポリシー（verify）でスナップショットを検証する。
pub fn assert_snapshot(name: impl Into<String>, value: impl Into<String>) -> TestResult {
    assert_snapshot_with(SnapshotPolicy::verify(), name, value)
}

/// ポリシー指定でスナップショットを検証する。
pub fn assert_snapshot_with(
    policy: SnapshotPolicy,
    name: impl Into<String>,
    value: impl Into<String>,
) -> TestResult {
    let name = name.into();
    let mut value = value.into();
    if policy.normalize {
        value = normalize_snapshot(&value);
    }
    if value.len() > policy.max_bytes {
        return Err(TestError::new(
            TestErrorKind::HarnessFailure,
            format!(
                "snapshot size exceeded: {} bytes (max {})",
                value.len(),
                policy.max_bytes
            ),
        )
        .with_context("snapshot.name", name));
    }

    let mut snapshots = SNAPSHOTS
        .lock()
        .map_err(|_| TestError::new(TestErrorKind::HarnessFailure, "snapshot lock poisoned"))?;
    match policy.mode {
        SnapshotMode::Verify => verify_snapshot(&mut snapshots, &name, &value),
        SnapshotMode::Update => update_snapshot(&mut snapshots, &name, &value),
        SnapshotMode::Record => record_snapshot(&mut snapshots, &name, &value),
    }
}

/// テストブロック相当の実行関数。
pub fn test(name: impl Into<String>, body: impl Fn() -> TestResult) -> TestResult {
    test_with(SnapshotPolicy::verify(), name, body)
}

/// テストブロックをポリシー付きで実行する。
pub fn test_with(
    _policy: SnapshotPolicy,
    name: impl Into<String>,
    body: impl Fn() -> TestResult,
) -> TestResult {
    let name = name.into();
    body().map_err(|err| {
        let err = err.with_case_name(name);
        record_test_diagnostic(&err);
        err
    })
}

/// テーブル駆動テストを実行する。
pub fn table_test<T>(cases: &[TableCase<T>], render: impl Fn(&T) -> String) -> TestResult {
    for (index, case) in cases.iter().enumerate() {
        let actual = render(&case.input);
        if actual != case.expected {
            return Err(
                TestError::new(TestErrorKind::AssertionFailed, "table_test mismatch")
                    .with_context("case_index", index.to_string())
                    .with_context("expected", case.expected.clone())
                    .with_context("actual", actual),
            );
        }
    }
    Ok(())
}

/// ファジング実行（最小版）。
pub fn fuzz_bytes(config: &FuzzConfig, f: impl Fn(&[u8]) -> TestResult + UnwindSafe) -> TestResult {
    let mut generator = FuzzGenerator::new(&config.seed);
    let max_cases = config.max_cases.max(1);
    let max_bytes = config.max_bytes.max(1);
    for _ in 0..max_cases {
        let bytes = generator.next_bytes(max_bytes);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&bytes)));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => return Err(err),
            Err(_) => {
                return Err(TestError::new(
                    TestErrorKind::FuzzCrash,
                    "fuzz case panicked",
                ))
            }
        }
    }
    Ok(())
}

/// ファジング設定。
#[derive(Clone, Debug)]
pub struct FuzzConfig {
    pub seed: Vec<u8>,
    pub max_cases: usize,
    pub max_bytes: usize,
}

#[derive(Clone, Debug)]
struct SnapshotEntry {
    value: String,
    hash: u64,
}

pub(crate) fn normalize_snapshot(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn verify_snapshot(store: &mut SnapshotStore, name: &str, value: &str) -> TestResult {
    match store.get(name) {
        Some(entry) if entry.value == value => Ok(()),
        Some(entry) => Err(
            TestError::new(TestErrorKind::SnapshotMismatch, "snapshot mismatch")
                .with_context("snapshot.name", name)
                .with_context("snapshot.expected_hash", entry.hash.to_string())
                .with_context("snapshot.actual_hash", snapshot_hash(value).to_string()),
        ),
        None => Err(
            TestError::new(TestErrorKind::SnapshotMissing, "snapshot missing")
                .with_context("snapshot.name", name),
        ),
    }
}

fn record_snapshot(store: &mut SnapshotStore, name: &str, value: &str) -> TestResult {
    if store.contains_key(name) {
        return verify_snapshot(store, name, value);
    }
    let hash = snapshot_hash(value);
    store.insert(
        name.to_string(),
        SnapshotEntry {
            value: value.to_string(),
            hash,
        },
    );
    record_snapshot_updated(name, hash, SnapshotMode::Record, value.len());
    Ok(())
}

fn update_snapshot(store: &mut SnapshotStore, name: &str, value: &str) -> TestResult {
    let hash = snapshot_hash(value);
    store.insert(
        name.to_string(),
        SnapshotEntry {
            value: value.to_string(),
            hash,
        },
    );
    record_snapshot_updated(name, hash, SnapshotMode::Update, value.len());
    Ok(())
}

pub(crate) fn snapshot_hash(value: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn record_snapshot_updated(name: &str, hash: u64, mode: SnapshotMode, bytes: usize) {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());
    let mut metadata = JsonMap::new();
    metadata.insert(
        "event.kind".into(),
        Value::String(AuditEventKind::SnapshotUpdated.as_str().into_owned()),
    );
    metadata.insert("event.domain".into(), Value::String("test".into()));
    metadata.insert("snapshot.name".into(), Value::String(name.to_string()));
    metadata.insert("snapshot.hash".into(), Value::String(hash.to_string()));
    metadata.insert(
        "snapshot.mode".into(),
        Value::String(
            match mode {
                SnapshotMode::Verify => "verify",
                SnapshotMode::Update => "update",
                SnapshotMode::Record => "record",
            }
            .into(),
        ),
    );
    metadata.insert("snapshot.bytes".into(), Value::String(bytes.to_string()));
    let envelope = AuditEnvelope::from_parts(metadata, None, None, Some("core.test".into()));
    let event = AuditEvent::new(timestamp, envelope);
    let mut events = TEST_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    events.push(event);
}

pub(crate) fn record_test_diagnostic(error: &TestError) {
    let diagnostic = error.into_diagnostic();
    let mut diagnostics = TEST_DIAGNOSTICS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    diagnostics.push(diagnostic);
}

/// 記録済みのテスト診断を取得してクリアする。
pub fn take_test_diagnostics() -> Vec<GuardDiagnostic> {
    let mut diagnostics = TEST_DIAGNOSTICS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let drained = diagnostics.clone();
    diagnostics.clear();
    drained
}

/// 記録済みの監査イベントを取得してクリアする。
pub fn take_test_audit_events() -> Vec<AuditEvent> {
    let mut events = TEST_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let drained = events.clone();
    events.clear();
    drained
}

struct FuzzGenerator {
    state: u64,
}

impl FuzzGenerator {
    fn new(seed: &[u8]) -> Self {
        let mut state = 0u64;
        for (idx, byte) in seed.iter().enumerate() {
            let shift = (idx % 8) * 8;
            state ^= (*byte as u64) << shift;
        }
        if state == 0 {
            state = 0x6f_72_65_6d_6c;
        }
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn next_bytes(&mut self, max_bytes: usize) -> Vec<u8> {
        let len = (self.next_u64() % max_bytes as u64) as usize;
        let mut out = vec![0u8; len];
        for chunk in out.chunks_mut(8) {
            let value = self.next_u64().to_le_bytes();
            let copy_len = chunk.len();
            chunk.copy_from_slice(&value[..copy_len]);
        }
        out
    }
}
