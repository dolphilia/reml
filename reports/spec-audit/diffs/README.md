# diffs - `rust-gap` 差分メモ

- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md#phase28-diff-class` で `rust-gap` に分類した差分 ID ごとに Markdown を追加する。
- ファイル命名: `<ID>-<chapter>-rust-gap.md`（例: `SYNTAX-003-ch1-rust-gap.md`）。
- 記録内容: 症状, 再現手順（Rust コマンド）, 期待結果, 現状結果, 対応ステータス, リンク。
- 完了したら `docs/notes/spec-integrity-audit-checklist.md#rust-gap` の表に `done` を記載し、差分メモに `Resolved:` セクションを設ける。
- `SYNTAX-002-ch1-rust-gap.md`（2025-11-17 追加）はテンプレート例。Module/Use 受理や `TraceEvent` など、仕様で要求される観測点をどのように証跡化するかを記載している。
- `SYNTAX-003-ch1-rust-gap.md`（2025-11-18 追加）は effect handler/operation/perform/do の受理ログと dual-write 証跡をまとめ、`ExprParser` への `trace_id = syntax:expr-*` 追加や `block_scope` / `effect_handler` サンプルの JSON を参照している。
