# Reml テストガイド（Core.Test）

> `Core.Test` を使って DSL のゴールデン/スナップショットを維持するための最小ガイド。

## 1. 目的
- DSL の解析結果や出力を **安定したスナップショット** として管理する。
- 診断 JSON を固定し、Phase 4 の回帰と同期する。

参照: [3-11 Core Test](../spec/3-11-core-test.md)

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

## 3. スナップショット更新ルール
- `update` モードは **破壊的変更時のみ** 使用する。
- `verify` は差分があれば失敗、`record` は未存在時のみ作成する。
- `snapshot.name` は `phase4-scenario-matrix.csv` の `scenario_id` と一致させる。
- 診断スナップショットは `Diagnostic.code` → `span.start.line/column` の順で安定化し、`run_id`/`timestamp` は比較対象から除外する。

## 4. 回帰への接続
- `examples/practical/core_test/` と `expected/practical/core_test/` をセットで追加する。
- 実行ログは `reports/spec-audit/ch4/logs/stdlib-test-*.md` に保存する。
