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

## 参考
- OCaml ベースライン: `reports/dual-write/front-end/w4-diagnostics/baseline/`
- diag ハーネス: `scripts/poc_dualwrite_compare.sh --mode diag`, `scripts/dualwrite_summary_report.py --diag-table`
