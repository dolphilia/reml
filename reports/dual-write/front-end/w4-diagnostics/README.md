# W4 診断互換試験レポート

`reports/dual-write/front-end/w4-diagnostics/` には diag モードで取得した成果物を Run ID ごとに保存する。ケース定義は `docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt`、カテゴリ別の状況は `docs/plans/rust-migration/appendix/w4-diagnostic-case-matrix.md` を参照。

## 実行手順メモ
1. `scripts/poc_dualwrite_compare.sh --mode diag --run-id <label> --cases docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt`
2. `scripts/dualwrite_summary_report.py <run_dir> --diag-table <tmp.md> --update-diag-readme reports/dual-write/front-end/w4-diagnostics/README.md`
3. 必要に応じて `baseline/` データ（OCaml 側ゲート）と比較し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の TODO を更新する。

## ケースサマリ
<!-- DIAG_TABLE_START -->

| case | gating | schema | metrics | diag_match | parser_audit (ocaml/rust) | parser_expected (ocaml/rust) | stream_outcome (ocaml/rust) | effects_regressions (ocaml/rust) | diag_counts (ocaml/rust) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| recover_else_without_if | ❌ | ✅ | ❌ | ❌ | 1.000 / - | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 0 |
| recover_lambda_body | ❌ | ✅ | ❌ | ❌ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 2 |
| recover_missing_semicolon | ❌ | ✅ | ❌ | ✅ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 1 |
| recover_missing_tuple_comma | ❌ | ✅ | ❌ | ✅ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 1 |
| recover_unclosed_block | ❌ | ✅ | ❌ | ✅ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 1 |
| type_condition_bool | ❌ | ✅ | ❌ | ❌ | 1.000 / 0.000 | 0.000 / 0.000 | - / - | 0 / 0 | 0 / 1 |

<!-- DIAG_TABLE_END -->

## 追加ケース（DIAG-RUST-05/06/07）
- **ストリーミング**: `stream_pending_resume`, `stream_backpressure_hint`, `stream_checkpoint_drift` を `docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` に登録。CLI 実行時は `--streaming --stream-resume-hint diag-w4 --stream-flow-policy auto` を付与し、`parser.stream.*` メトリクスが計測可能なケースのみ `collect-iterator-audit-metrics.py --section streaming` を実行する。
- **効果 / Capability**: `type_condition_literal_bool`（bool 条件リテラル簡易版）、`effect_residual_leak`, `effect_stage_cli_override`、および `ffi_stage_messagebox`, `ffi_ownership_mismatch`, `ffi_async_dispatch` を追加。`--experimental-effects --type-row-mode dual-write --effect-stage beta --runtime-capabilities tooling/audit-store/capabilities/dev.json` を既定フラグとして記録した。
- **CLI / LSP**: `cli_packrat_switch`, `cli_trace_toggle`, `cli_merge_warnings` を CLI RunConfig 用に登録し、同一入力を利用する LSP フィクスチャ（`lsp_hover_internal_error`, `lsp_diagnostic_stream`, `lsp_workspace_config`）を `tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-*.json` と関連付けた。`npm run ci --prefix tooling/lsp/tests/client_compat` のログを `reports/dual-write/front-end/w4-diagnostics/<run>/lsp/` に保存する。

## 参考
- OCaml ベースライン: `reports/dual-write/front-end/w4-diagnostics/baseline/`
- diag ハーネス: `scripts/poc_dualwrite_compare.sh --mode diag`, `scripts/dualwrite_summary_report.py --diag-table`
