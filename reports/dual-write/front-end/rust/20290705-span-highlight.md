# Run ID: 20290705-span-highlight

- **コマンド**: `cargo test --manifest-path compiler/rust/runtime/Cargo.toml span_highlight`
- **目的**: `Core.Text` の `span_highlight` 実装がマルチバイト書記素を正しく処理し、CLI/JSON のハイライト情報に利用できることを確認する。
- **結果**: テスト 2 件 (`expect_span_highlight`, `span_highlight_handles_grapheme_clusters`) がいずれも成功。新規警告は既存の Runtime crate の未使用インポートのみで機能回帰は無し。
- **関連 KPI**: `diagnostic.span_highlight_pass_rate = 1.0`
- **参照**: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md#diagnostic-span-highlight-pass-rate`
