# 3.11 Core Test

> 目的：DSL 開発で必要な統合テスト・ゴールデンテスト・ファジングの基盤を標準化し、診断と監査の一貫性を保つ。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 4 対象） |
| 効果タグ | `effect {io}`, `effect {audit}` |
| 依存モジュール | `Core.Prelude`, `Core.Diagnostics`, `Core.Text` |
| 相互参照 | [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), Guides: [testing](../guides/testing.md) |

## 1. 基本概念

`Core.Test` は DSL の **出力の安定化** と **診断ログの再現** を目的に、テスト記述とスナップショット管理を提供する。テストの結果は `Result` として返し、失敗時は `TestError` へ集約する。

## 2. 型と API

```reml
pub type TestError = {
  kind: TestErrorKind,
  message: Str,
  context: Map<Str, Str>,
}

pub enum TestErrorKind =
  | AssertionFailed
  | SnapshotMismatch
  | SnapshotMissing
  | HarnessFailure
  | FuzzCrash

pub type SnapshotPolicy = {
  mode: Str,          // "verify" | "update" | "record"
  normalize: Bool,
  max_bytes: Int,
}

pub type TestCase = {
  name: Str,
  body: fn() -> Result<(), TestError>,
}

fn assert_eq<T: Eq>(actual: T, expected: T) -> Result<(), TestError>
fn assert_snapshot(name: Str, value: Str) -> Result<(), TestError>
fn assert_snapshot_with(policy: SnapshotPolicy, name: Str, value: Str) -> Result<(), TestError>
fn test(name: Str, body: fn() -> Result<(), TestError>) -> Result<(), TestError>
fn test_with(policy: SnapshotPolicy, name: Str, body: fn() -> Result<(), TestError>) -> Result<(), TestError>
```

### 2.1 テストブロック

```reml
test "core_test_basic" {
  Test.assert_snapshot("core_test_basic", "alpha")?
}
```

- `test "name" { ... }` は `Test.test` と同等の `Result` を返すテストブロック構文とする。
- 失敗時は `TestError` へ集約し、`test.failed` 診断へ橋渡しする。

## 3. スナップショット更新ポリシーと安定化

- `mode = "verify"` は差分があれば `SnapshotMismatch`、`"update"` は既存を更新、`"record"` は未存在時のみ作成する。
- `normalize = true` の場合は改行を `\n` に統一し、末尾の連続空白は保持する。
- `max_bytes` を超える場合は `SnapshotMissing` ではなく `HarnessFailure` とする。
- `snapshot.name` は `phase4-scenario-matrix.csv` の `scenario_id` と一致させる。
- 診断スナップショットは `Diagnostic.code` → `span.start.line/column` の順で安定化し、`run_id`/`timestamp` は比較対象から除外する。
- パスはワークスペース相対表記へ正規化し、環境差異での揺れを抑制する。

## 4. テーブル駆動テスト

```reml
pub type TableCase<T> = { input: T, expected: Str }

fn table_test<T>(cases: List<TableCase<T>>, render: fn(T) -> Str) -> Result<(), TestError>
```

- `render` が返す文字列を `expected` と比較する。
- 診断差分の再現を優先する場合は `render` 内で JSON を組み立ててもよい。

## 5. ファジングと再現性

```reml
pub type FuzzConfig = {
  seed: Bytes,
  max_cases: Int,
  max_bytes: Int,
}

fn fuzz_bytes(config: FuzzConfig, f: fn(Bytes) -> Result<(), TestError>) -> Result<(), TestError>
```

- `seed` を監査ログに記録し、再現可能性を確保する。
- `FuzzCrash` は `Core.Diagnostics` の `AuditEvent::TestFuzzCrash` と同期する。

## 6. 診断と監査

- 失敗時は `Diagnostic.code = "test.failed"` を既定とし、`extensions["test"].case_name` を必須とする。
- スナップショット更新時は `AuditEvent::SnapshotUpdated` を発行し、`snapshot.name` / `snapshot.hash` を記録する。

## 7. 例

```reml
use Core.Test

fn main() -> Str {
  match Test.assert_snapshot("core_test_basic", "alpha") {
    Ok(_) => "snapshot:ok",
    Err(_) => "snapshot:error",
  }
}
```

```reml
use Core.Test

fn main() -> Str {
  let cases = [
    { input: "alpha", expected: "alpha" },
    { input: "beta", expected: "beta" },
  ]

  match Test.table_test(cases, |value| value) {
    Ok(_) => "table:ok",
    Err(_) => "table:error",
  }
}
```
