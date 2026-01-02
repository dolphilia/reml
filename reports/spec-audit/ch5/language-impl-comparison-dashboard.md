# language_impl_comparison スイート実行レポート

- 実行時刻: 2025-12-18 07:44:29Z
- 対象シナリオ: 1 件 / 成功 1 件 / 失敗 0 件
- 入力ソース: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`

| Scenario | File | 期待 Diagnostics | 実際 Diagnostics | Exit | 判定 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| `CH2-PARSE-501` | `examples/language-impl-comparison/reml/basic_interpreter_combinator.reml` | — | — | 0 | ✅ pass | Core.Parse コンビネータによる BASIC 風インタープリタの統合例。Packrat と chainl1/chainr1 が CLI で動作することを検証する。 |
