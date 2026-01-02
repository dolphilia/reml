# module_parser-20251119 Dual-write 比較

- Date: 2025-11-19 11:05 JST
- CI_RUN_ID: `rust-frontend-w37-20251119.1`
- Commit: `f9e10ae676bca22ed8a41e96d79f667310274990`
- 実行スクリプト: `scripts/poc_dualwrite_compare.sh use_nested`, `scripts/poc_dualwrite_compare.sh effect_handler`

## 比較結果サマリ

| サンプル | OCaml 診断 | Rust 診断 | 差分 | 備考 |
| --- | --- | --- | --- | --- |
| `use_nested.reml` | 0 | 0 | 差分 0 | `TraceEvent::ModuleStageEntered`/`TraceEvent::ModuleDeclAccepted` を併せて確認。 |
| `effect_handler.reml` | 0 | 0 | 差分 0 | `operation log(args, resume)` の `resume` 型検証結果が一致。 |
| `block_scope.reml` | 0 | 0 | 差分 0 | module_parser 経由でも `ExprParser` の結果が再現できることを確認。 |

## スクリプトログ

```
scripts/poc_dualwrite_compare.sh use_nested \
  --ci-run rust-frontend-w37-20251119.1 \
  --store reports/spec-audit/ch1/use_nested-20251119-dualwrite.log

scripts/poc_dualwrite_compare.sh effect_handler \
  --ci-run rust-frontend-w37-20251119.1 \
  --store reports/spec-audit/ch1/effect_handler-20251119-dualwrite.log
```

- `use_nested-20251119-dualwrite.log` と `effect_handler-20251119-dualwrite.log` の差分 0 行を確認した。
- `docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` へ `Module Parser Acceptance` チェックを追記済み。

## 次アクション
- `docs/notes/process/spec-integrity-audit-checklist.md` の `SYNTAX-002/module_parser` 行を `In Review (P2-8 W38)` に設定。
- `reports/spec-audit/ch1/2025-11-17-syntax-samples.md` へ 2025-11-19 分のログ概要を追加。
- `compiler/rust/frontend/tests/parser.rs` のケースを CI に組み込み、`cargo test ... parser::module` を Phase 2-8 の差分ゲートに昇格させる。
