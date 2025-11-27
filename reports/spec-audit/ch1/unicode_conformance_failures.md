# Unicode Conformance 失敗ログ

Unicode Conformance テスト（UAX #29 / UAX #15）の実行結果と失敗ケースを追跡する。新しい失敗が発生した場合はテーブルへ追記し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にもリスクとして登録する。

| 日付 | コマンド | データセット | 状態 | 備考 |
| --- | --- | --- | --- | --- |
| 2027-03-31 | `cargo test unicode_conformance --features unicode_full --manifest-path compiler/rust/runtime/Cargo.toml` | `tests/data/unicode/UAX29/GraphemeBreakTest-15.1.0.txt`, `tests/data/unicode/UAX15/NormalizationTest-15.1.0.txt` | ✅ Pass | Grapheme/Normalization の両テストが 100% 合格。失敗ケースなし。 |

- 失敗時は上表に `❌ Fail` と原因を記載し、再現条件（Unicode 版、フォーム、行番号など）を `details/` フォルダへ Markdown で保存する。
- `reports/text-normalization-metrics.json` や `reports/spec-audit/ch1/core_text_grapheme_stats.json` など KPI ソースが更新された場合、本ログからも参照できるよう備考に Run ID を記す。
