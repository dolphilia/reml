//! Core.Test 仕様の最小実装。
//! スナップショットの保持はプロセス内メモリに限定し、IO 連携は後続フェーズで実装する。

use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::panic::UnwindSafe;
use std::sync::Mutex;

const DEFAULT_SNAPSHOT_MAX_BYTES: usize = 1024 * 1024;

type SnapshotStore = HashMap<String, SnapshotEntry>;

static SNAPSHOTS: Lazy<Mutex<SnapshotStore>> = Lazy::new(|| Mutex::new(HashMap::new()));

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
    body().map_err(|err| err.with_case_name(name))
}

/// テーブル駆動テストを実行する。
pub fn table_test<T>(cases: &[TableCase<T>], render: impl Fn(&T) -> String) -> TestResult {
    for (index, case) in cases.iter().enumerate() {
        let actual = render(&case.input);
        if actual != case.expected {
            return Err(TestError::new(
                TestErrorKind::AssertionFailed,
                "table_test mismatch",
            )
            .with_context("case_index", index.to_string())
            .with_context("expected", case.expected.clone())
            .with_context("actual", actual));
        }
    }
    Ok(())
}

/// ファジング実行（最小版）。
pub fn fuzz_bytes(
    config: &FuzzConfig,
    f: impl Fn(&[u8]) -> TestResult + UnwindSafe,
) -> TestResult {
    let mut generator = FuzzGenerator::new(&config.seed);
    let max_cases = config.max_cases.max(1);
    let max_bytes = config.max_bytes.max(1);
    for _ in 0..max_cases {
        let bytes = generator.next_bytes(max_bytes);
        let result = std::panic::catch_unwind(|| f(&bytes));
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

fn normalize_snapshot(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn verify_snapshot(store: &mut SnapshotStore, name: &str, value: &str) -> TestResult {
    match store.get(name) {
        Some(entry) if entry.value == value => Ok(()),
        Some(entry) => Err(TestError::new(
            TestErrorKind::SnapshotMismatch,
            "snapshot mismatch",
        )
        .with_context("snapshot.name", name)
        .with_context("snapshot.expected_hash", entry.hash.to_string())
        .with_context("snapshot.actual_hash", snapshot_hash(value).to_string())),
        None => Err(TestError::new(
            TestErrorKind::SnapshotMissing,
            "snapshot missing",
        )
        .with_context("snapshot.name", name)),
    }
}

fn record_snapshot(store: &mut SnapshotStore, name: &str, value: &str) -> TestResult {
    if store.contains_key(name) {
        return verify_snapshot(store, name, value);
    }
    store.insert(
        name.to_string(),
        SnapshotEntry {
            value: value.to_string(),
            hash: snapshot_hash(value),
        },
    );
    Ok(())
}

fn update_snapshot(store: &mut SnapshotStore, name: &str, value: &str) -> TestResult {
    store.insert(
        name.to_string(),
        SnapshotEntry {
            value: value.to_string(),
            hash: snapshot_hash(value),
        },
    );
    Ok(())
}

fn snapshot_hash(value: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
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
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
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
