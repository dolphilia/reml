# ドキュメント更新ログ（Core.Text 関連）

## 目的
Core.Text/Unicode 関連の文書更新を時系列で追跡し、`docs/plans/bootstrap-roadmap/checklists/doc-sync-text.md` と整合させる。

## ログ
| 日付 | 文書/セクション | 変更概要 | リンク | 状況 |
| --- | --- | --- | --- | --- |
| 2025-11-21 | `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` | 作業ブレークダウンへ詳細ステップを追記 | (current) | 完了 |
| 2027-03-30 | `docs/spec/3-3-core-text-unicode.md` / `examples/core-text` | §9 へサンプル脚注を追加し、`text_unicode.reml` と `expected/text_unicode.*.golden` を整備 | examples/core-text/README.md | 完了 |
| 2027-03-30 | `README.md` / `3-0-phase3-self-host.md` / `docs/plans/bootstrap-roadmap/README.md` | Core.Text ハイライトを追記し、Phase 3 の進捗に `examples/core-text` と `reports/spec-audit/ch1/core_text_examples-20270330.md` をリンク | README.md | 完了 |
| 2025-12-20 | `docs/plans/bootstrap-roadmap/4-1-dsl-lite-profile-plan.md` | `remlc new` の CLI 検証ログを追加 | docs/plans/bootstrap-roadmap/4-1-dsl-lite-profile-plan.md | 完了 |
| 2025-12-19 | `reports/spec-audit/ch5/logs/stdlib-test-dsl-template.md` / `examples/practical/core_test/dsl/ast_matcher_basic.reml` | DSL Test Kit の CLI 実行ログとサンプルを更新 | reports/spec-audit/ch5/logs/stdlib-test-dsl-template.md | 完了 |
| 2025-12-19 | `compiler/frontend/src/parser/mod.rs` / `examples/practical/core_test/dsl/ast_matcher_basic.reml` | `test_parser { case ... }` 糖衣構文の導入とサンプル更新 | compiler/frontend/src/parser/mod.rs | 完了 |
| 2025-12-19 | `reports/spec-audit/ch5/logs/stdlib-test-dsl-template.md` | DSL Test Kit の CLI 実行ログ（糖衣構文更新後）を追記 | reports/spec-audit/ch5/logs/stdlib-test-dsl-template.md | 完了 |
| 2025-12-19 | `examples/practical/core_test/dsl/error_expectation_basic.reml` | error_expectation の DSL を糖衣構文へ移行 | examples/practical/core_test/dsl/error_expectation_basic.reml | 完了 |
| 2025-12-19 | `reports/spec-audit/ch5/logs/stdlib-test-dsl-template.md` | DSL Test Kit の CLI 実行ログ（error_expectation 更新後）を更新 | reports/spec-audit/ch5/logs/stdlib-test-dsl-template.md | 完了 |

## TODO
- [ ] Core.Text サンプル (`examples/core-text`) 追加後にログへ記入。
- [ ] README/ガイド反映時に ID を採番してチェックリストと双方向リンクする。
