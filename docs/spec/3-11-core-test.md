# 3.11 Core Test

> 目的：DSL 開発で必要な統合テスト・ゴールデンテスト・ファジングの基盤を標準化し、診断と監査の一貫性を保つ。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 4 対象） |
| 効果タグ | `effect {io}`, `effect {audit}` |
| 依存モジュール | `Core.Prelude`, `Core.Diagnostics`, `Core.Text` |
| 相互参照 | [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), Guides: [testing](../guides/tooling/testing.md) |

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
use Core.Test

fn main() -> Result<(), TestError> {
  test "core_test_basic" {
    Test.assert_snapshot("core_test_basic", "alpha")?
  }
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
- Phase 4 の `examples/practical/core_test/` は暫定的に CLI JSON 出力を `expected/` に合わせる（Runtime の stdout 経路整備後に `snapshot:ok` へ戻す）。

### 3.1 ゴールデンファイル命名（golden_case）

- `golden_case` は `case_id` を基準に入力と期待値を 3 点セットで管理する。
- `case_id` は `snapshot.name` と `phase4-scenario-matrix.csv` の `scenario_id` に一致させる。
- 入力は `examples/**/golden/{case_id}.input`、期待値は `expected/**/golden/{case_id}.ast` / `expected/**/golden/{case_id}.error` に保存する。

## 4. テーブル駆動テスト

```reml
pub type TestError

pub type TableCase<T> = { input: T, expected: Str }

fn table_test<T>(cases: List<TableCase<T>>, render: fn(T) -> Str) -> Result<(), TestError>
```

- `render` が返す文字列を `expected` と比較する。
- 診断差分の再現を優先する場合は `render` 内で JSON を組み立ててもよい。

## 5. ファジングと再現性

```reml
pub type Bytes
pub type TestError

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
- スナップショット更新時は `AuditEvent::SnapshotUpdated`（イベント名は `snapshot.updated`）を発行し、`snapshot.name` / `snapshot.hash` / `snapshot.mode` / `snapshot.bytes` を記録する。

## 7. DSL Test Kit（Core.Test.Dsl）

`Core.Test.Dsl` は DSL パーサー向けのテスト記述を簡潔化するための糖衣構文と Matcher 群を提供する。`Core.Test` のスナップショット/診断ポリシーと同一の更新規則に従う。

### 7.1 最小構文

```reml
use Core.Parse
use Core.Test
use Core.Test.Dsl

fn main() -> Result<(), TestError> {
  let my_parser: Parser<Any> = todo
  test_parser(my_parser) {
    case "1 + 2" => Ast(Pattern("Add(Int(1), Int(2))"))
    case "1 + " => Error({
      code: "parser.unexpected_eof",
      at: Some(Offset(4)),
      message: None,
    })
    case "fn main() {}" => Ast(Pattern("Func(name=\"main\", ...)"))
  }
}
```

Rust Frontend では `test_parser(parser) { ... }` のブロック構文を受理する。`case` は `case "source" => <expect>` または `case "name": "source" => <expect>` を許可する。

### 7.2 型とシグネチャ（Core.Parse / Core.Test との整合）

```reml
pub type Parser<T>
pub type TestError

pub enum AstMatcher<T> =
  | Pattern(Str)
  | Record(List<(Str, AstMatcher<T>)>)
  | List(List<AstMatcher<T>>)

pub type DslCase<T> = {
  name: Option<Str>,
  source: Str,
  expect: DslExpectation<T>,
}

pub enum DslExpectation<T> =
  | Ast(AstMatcher<T>)
  | Error(ErrorExpectation)

pub type ErrorExpectation = {
  code: Str,
  at: Option<AtSpec>,
  message: Option<Str>,
}

pub enum AtSpec =
  | Offset(Int)
  | LineCol(line: Int, col: Int)

fn test_parser<T>(parser: Parser<T>, cases: List<DslCase<T>>) -> Result<(), TestError>
```

- `test_parser` は `Core.Parse` が返す `ParseResult<T>`（2-1）を入力にし、`Result<(), TestError>` で失敗を返す。
- `AtSpec.Offset` は `Diagnostic.at.byte_start`（0-origin）、`AtSpec.LineCol` は `Diagnostic.at.line_start` / `col_start`（1-origin）と一致判定する。
- `DslExpectation::Ast` は `ParseResult.value = Some(_)` かつ `diagnostics` に `Severity::Error` が無い場合のみ評価する。
- `DslExpectation::Error` は `ParseResult.diagnostics` から `Severity::Error` の診断を抽出し、`ErrorExpectation` と突き合わせる。

### 7.3 Matcher 仕様（最小セット）
- `...` は構造的部分一致を示し、未指定フィールドを無視する。
- `List`/`Record` は順序/キー一致を必須とし、欠落は `AssertionFailed` とする。
- `Option`/`Result` は `Some(...)` / `Ok(...)` を簡略記法として許可する。

```reml
use Core.Parse
use Core.Test
use Core.Test.Dsl

fn main() -> Result<(), TestError> {
  let my_parser: Parser<Any> = todo
  test_parser(my_parser) {
    case "1 + 2" => Ast(Pattern("...Add(Int(1), Int(2))..."))
    case "1 + 3" => Ast(Record([
      ("tag", Pattern("\"expr\"")),
      ("items", List([Pattern("Add(Int(1), Int(3))")])),
    ]))
  }
}
```

### 7.4 Error Expectation
- `code`: 診断コード（必須）。`Diagnostic.code` または `Diagnostic.codes` と一致した場合に合格。
- `at`: 文字位置（`Offset(Int)`）または `LineCol(line: Int, col: Int)`。
- `message`: 部分一致（省略可）。

`Error(...)` は `Core.Diagnostics` の `Diagnostic` と突き合わせ、失敗時は `test.failed` へ集約する。

### 7.5 診断コード最小セットと命名
- 診断コードは 2-5 §C-1 の `message_key` と同じく **小文字ドット区切り**を採用し、未登録コードは 3-6 §2 の `DiagnosticCatalog` に登録してから利用する。
- `Core.Test.Dsl` が参照する最小セットは以下とする。
  - `parser.syntax.expected_tokens`: 既定の構文期待値エラー（2-5 §B-5 の規約に従う）。
  - `parser.unexpected_eof`: 入力終端で期待集合が満たせない場合の EOF 失敗。`at` は入力末尾を指すこと。

## 8. 例

```reml
use Core.Test

fn main() -> Str {
  let outcome = Test.assert_snapshot("core_test_basic", "alpha")
  match outcome with
    | Ok(_) -> "snapshot:ok"
    | Err(_) -> "snapshot:error"
}
```

```reml
use Core.Test

fn main() -> Str {
  let cases = [
    { input: "alpha", expected: "alpha" },
    { input: "beta", expected: "beta" },
  ]

  let outcome = Test.table_test(cases, |value| value)
  match outcome with
    | Ok(_) -> "table:ok"
    | Err(_) -> "table:error"
}
```
