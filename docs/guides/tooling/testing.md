# Reml テストガイド（Core.Test）

> `Core.Test` を使って DSL のゴールデン/スナップショットを維持するための最小ガイド。

## 1. 目的
- DSL の解析結果や出力を **安定したスナップショット** として管理する。
- 診断 JSON を固定し、Phase 4 の回帰と同期する。

参照: [3-11 Core Test](../../spec/3-11-core-test.md)

## 2. 最小例

```reml
use Core.Test

fn main() -> Str {
  let outcome = Test.assert_snapshot("core_test_basic", "alpha")
  match outcome with
    | Ok(_) -> "snapshot:ok"
    | Err(_) -> "snapshot:error"
  }
}
```

```reml
use Core.Test

fn main() -> Str {
  let outcome = test "core_test_basic" {
    Test.assert_snapshot("core_test_basic", "alpha")?
    Ok(())
  }
  match outcome with
    | Ok(_) -> "snapshot:ok"
    | Err(_) -> "snapshot:error"
  }
}
```

## 3. DSL Test Kit の最小例

```reml
use Core.Test.Dsl

test_parser(my_parser) {
  case "1 + 2" => Add(Int(1), Int(2))
  case "1 + " => Error(code="parser.unexpected_eof", at=4)
}
```

### 3.1 AtSpec と診断コードの最小セット
- `at` は `Offset(Int)`（0-origin, `Diagnostic.at.byte_start`）または `LineCol(line: Int, col: Int)`（1-origin, `Diagnostic.at.line_start`/`col_start`）を指定する。
- 診断コードは **小文字ドット区切り**を採用し、未登録コードは `DiagnosticCatalog` に登録してから利用する。
- Test DSL が参照する最小セットは以下。
  - `parser.syntax.expected_tokens`: 既定の構文期待値エラー。
  - `parser.unexpected_eof`: 入力終端で期待集合が満たせない場合の EOF 失敗（`at` は入力末尾）。

## 4. スナップショット更新ルール
- `update` モードは **破壊的変更時のみ** 使用する。
- `verify` は差分があれば失敗、`record` は未存在時のみ作成する。
- `snapshot.name` は `phase4-scenario-matrix.csv` の `scenario_id` と一致させる。
- 診断スナップショットは `Diagnostic.code` → `span.start.line/column` の順で安定化し、`run_id`/`timestamp` は比較対象から除外する。

## 5. ゴールデン運用と更新ポリシー
- 入力/期待値は `*.input`/`*.ast`/`*.error` の 3 点セットを基本とする。
- `*.ast` は AST のレンダリング結果、`*.error` は `Diagnostic` JSON を想定する。
- `case_id` は `snapshot.name` と `phase4-scenario-matrix.csv` の `scenario_id` に一致させる。
- 既定の配置は `examples/**/golden/{case_id}.input` と `expected/**/golden/{case_id}.ast` / `expected/**/golden/{case_id}.error`。
- `update` モードは互換性破壊時のみ使用し、`resolution_notes` に更新理由を残す。
- 期待値の差分確認は `snapshot.name` と `scenario_id` の一致を必須とする。
- `run_id`/`timestamp`/環境依存パスは比較対象から除外する。
- 更新時の監査イベントは `snapshot.updated` を使用し、`snapshot.name` / `snapshot.hash` / `snapshot.mode` / `snapshot.bytes` を記録する。

## 6. 回帰への接続
- `examples/practical/core_test/` と `expected/practical/core_test/` をセットで追加する。
- 実行ログは `reports/spec-audit/ch5/logs/stdlib-test-*.md` に保存する。
- Phase 4 の Core.Test サンプルは暫定的に CLI JSON 出力を `expected/` に合わせる（Runtime の stdout 経路整備後に `snapshot:ok` へ戻す）。
