# pattern-matching クロス実装差分メモ（CH1-MATCH/ACT）

> 状態: **凍結（無効）**  
> OCaml 実装の更新が停止しているため、本メモに基づく継続的なクロス実装差分追跡は現時点では行いません（`docs/plans/pattern-matching-improvement/1-2-match-ir-lowering-plan.md` の M5 を参照）。

本メモは `docs/plans/pattern-matching-improvement/1-2-match-ir-lowering-plan.md` の **M5（クロス実装チェック）** を実施するための記録用テンプレートです。

## 対象

- Active Pattern: `CH1-ACT-001..003`（`examples/spec_core/chapter1/active_patterns/`）
- Match/Pattern: `CH1-MATCH-007..018`（`examples/spec_core/chapter1/match_expr/`）

期待診断キー・既知の run_id 記録は `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` を正とします。

## 実行コマンド（例）

### Rust（基準）

```bash
compiler/rust/frontend/target/debug/reml_frontend --output json examples/spec_core/chapter1/match_expr/bnf-match-or-pattern-ok.reml
```

比較対象は `diagnostics[].code` と `exit_code`（`label`/`value`）とし、`run_id` 等の変動フィールドは比較から除外します。

### OCaml（比較対象）

環境により以下のいずれかを使用します。

```bash
compiler/ocaml/_build/default/src/main.exe examples/spec_core/chapter1/match_expr/bnf-match-or-pattern-ok.reml
```

または:

```bash
remlc-ocaml examples/spec_core/chapter1/match_expr/bnf-match-or-pattern-ok.reml
```

## 差分記録（テンプレート）

| Scenario | 入力 | 期待（診断キー） | Rust 実測 | OCaml 実測 | 判定（spec/impl/example） | コメント |
| --- | --- | --- | --- | --- | --- | --- |
| CH1-MATCH-007 | `...` | `[]` | `[]` | `[]` | - | - |

## 判定ルール（暫定）

- **spec_fix**: 両実装が診断を出すが、コード集合が仕様/マトリクス定義と衝突する（コードの集合が一致しない）。
- **impl_fix**: 片側のみ診断が出る、または exit code/severity がポリシーと矛盾する。
- **example_fix**: `.reml` の意図が仕様例から外れている、あるいは expected 更新のみで解消する。

## TODO

- [ ] OCaml CLI の JSON 出力形式（あれば）を特定し、抽出手順を固定する。
- [ ] `CH1-ACT-001..003` と `CH1-MATCH-007..018` を両実装で実行し、差分を表へ記録する。
- [ ] 差分が出た場合、仕様側（`docs/spec/1-1` / `1-5` / `2-5`）と計画側（`phase4-scenario-matrix.csv`）のどちらで吸収するかを決める。
