# ch0 - Chapter 0 監査ログ

- 対象: `docs/spec/0-0-overview.md`, `docs/spec/0-1-project-purpose.md`, `docs/spec/0-2-glossary.md`, `docs/spec/0-3-code-style-guide.md`, `docs/spec/README.md`。
- 保存物: リンクチェッカー出力 (`links.md`)、脚注更新差分、Rust CLI から参照する `--help` テキストのスクリーンショット。
- 手順: `scripts/ci-detect-regression.sh --mode links --docs docs/spec` を実行し、結果を貼付。リンク切れを検出した場合は `docs/plans/repository-restructure-plan.md` に Issue ID を追記。
- 更新責任者: Spec Core WG（#spec-core）。
