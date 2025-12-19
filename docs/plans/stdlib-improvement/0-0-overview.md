# 0.0 標準ライブラリ改善計画 概要

## 背景
- `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` を進める途中で、DSL 開発のライフサイクルを支える標準モジュールが不足していると判断した。
- `docs/notes/stdlib-improvement-proposal.md` で、テスト/CLI/整形/ドキュメント/LSP といった周辺領域のギャップが整理されている。
- Reml の価値観（`docs/spec/0-1-project-purpose.md`）に照らし、DSL ファーストを満たすには **実装前に仕様とツール支援の枠組み** を揃える必要がある。

## 目的
1. DSL の設計・検証・配布までをカバーする標準ライブラリ拡張を計画化し、Phase 4 の回帰に統合できる状態へ整える。
2. 仕様書（3-x）とガイド（docs/guides/）に反映する項目を明確化し、実装段階の抜け漏れを防ぐ。
3. 既存モジュール（Core.Parse/Core.IO/Core.Diagnostics など）との整合を保ち、追加モジュールの依存関係を明示する。

## スコープ
- **含む**: `Core.Test`, `Core.Cli`, `Core.Text.Pretty`, `Core.Doc`, `Core.Lsp` の仕様設計、API 設計、ガイド/サンプル更新計画、回帰計測の追加案。
- **含まない**: 実装作業（Rust/OCaml のコード改修）、CLI/ツール配布パッケージの実体構築、CI スクリプトの確定実装。

## 成功条件
- 各モジュールの API/診断/メトリクスが `docs/spec/3-x` に整理され、`docs/spec/3-0-core-library-overview.md` から参照できる。
- DSL 作者が **テスト・CLI・フォーマッタ・ドキュメント・IDE 支援** を標準 API で組み立てられる導線が計画書に記載されている。
- Phase 4 回帰で測定するべき追加指標とシナリオが明文化されている。

## 参照資料
- メモ: `docs/notes/stdlib-improvement-proposal.md`
- 仕様: `docs/spec/3-0-core-library-overview.md`, `docs/spec/3-3-core-text-unicode.md`, `docs/spec/3-5-core-io-path.md`, `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`
- 関連計画: `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md`
